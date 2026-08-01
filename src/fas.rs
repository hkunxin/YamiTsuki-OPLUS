use std::fs;

use crate::gpu::{GED_LOADING_PATH, GED_UTIL_PATH};

const GPU_BUSY: &str = "/sys/class/kgsl/kgsl-3d0/gpubusy";
const GPU_UTIL: &str = "/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage";

pub struct FasScheduler {
    history: [u32; 4],
    idx: usize,
}

fn bounded_percent(raw: &str) -> Option<u32> {
    let value = raw.split_whitespace().find_map(|part| part.parse::<u32>().ok())?;
    (value <= 100).then_some(value)
}

impl FasScheduler {
    pub fn new() -> Self {
        FasScheduler { history: [0; 4], idx: 0 }
    }

    /// Read the real GPU utilization percentage.
    /// GED reports utilization directly on the OPPO/MediaTek target.
    pub fn gpu_util(&self) -> u32 {
        if let Ok(raw) = fs::read_to_string(GED_UTIL_PATH) {
            if let Some(value) = bounded_percent(&raw) {
                return value;
            }
        }
        if let Ok(raw) = fs::read_to_string(GED_LOADING_PATH) {
            if let Some(value) = bounded_percent(&raw) {
                return value;
            }
        }
        if let Ok(raw) = fs::read_to_string(GPU_BUSY) {
            let mut values = raw.split_whitespace().filter_map(|part| part.parse::<u64>().ok());
            if let (Some(busy), Some(total)) = (values.next(), values.next()) {
                if total > 0 {
                    return ((busy.saturating_mul(100) / total).min(100)) as u32;
                }
            }
        }
        fs::read_to_string(GPU_UTIL).ok().and_then(|raw| bounded_percent(&raw)).unwrap_or(0)
    }

    pub fn update(&mut self) -> u32 {
        let curr = self.gpu_util();
        self.history[self.idx] = curr;
        self.idx = (self.idx + 1) % self.history.len();
        self.history.iter().sum::<u32>() / self.history.len() as u32
    }

    pub fn freq_scale(&self, avg_util: u32) -> f64 {
        if avg_util > 80 { 1.0 } else if avg_util > 60 { 0.85 } else if avg_util > 40 { 0.7 } else { 0.5 }
    }
}
