use std::fs;

use crate::gpu::{GED_CURRENT_FREQ_PATH, GED_LOADING_PATH, GED_UTIL_PATH};

const GPU_BUSY: &str = "/sys/class/kgsl/kgsl-3d0/gpubusy";
const GPU_UTIL: &str = "/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage";

pub struct FasScheduler {
    history: [u32; 4],
    idx: usize,
}

fn numbers(raw: &str) -> Vec<u64> {
    raw.split_whitespace().filter_map(|part| part.parse::<u64>().ok()).collect()
}

fn ged_util(raw: &str) -> Option<u32> {
    let values = numbers(raw);
    if values.len() >= 3 {
        // PLG110 reports three bucket values; use their weighted total.
        let total = values.iter().take(3).sum::<u64>();
        if total > 0 {
            return Some(((values[2].saturating_mul(100) / total).min(100)) as u32);
        }
    }
    values.first().copied().filter(|v| *v <= 100).map(|v| v as u32)
}

impl FasScheduler {
    pub fn new() -> Self { FasScheduler { history: [0; 4], idx: 0 } }

    pub fn gpu_util(&self) -> u32 {
        if let Ok(raw) = fs::read_to_string(GED_UTIL_PATH) {
            if let Some(value) = ged_util(&raw) { return value; }
        }
        if let Ok(raw) = fs::read_to_string(GED_LOADING_PATH) {
            let values = numbers(&raw);
            if values.len() >= 2 && values[1] > 0 {
                return ((values[0].saturating_mul(100) / values[1]).min(100)) as u32;
            }
        }
        if let Ok(raw) = fs::read_to_string(GPU_BUSY) {
            let values = numbers(&raw);
            if values.len() >= 2 && values[1] > 0 {
                return ((values[0].saturating_mul(100) / values[1]).min(100)) as u32;
            }
        }
        fs::read_to_string(GPU_UTIL).ok().and_then(|raw| ged_util(&raw)).unwrap_or(0)
    }

    pub fn current_gpu_freq(&self) -> u64 {
        // PLG110 GED format is "level frequency" and may report kHz-like
        // values, while devfreq cur_freq may be scaled by the vendor driver.
        let ged = fs::read_to_string(GED_CURRENT_FREQ_PATH).ok()
            .and_then(|raw| numbers(&raw).get(1).copied()).unwrap_or(0);
        let devfreq = [
            "/sys/devices/platform/soc/13000000.mali/devfreq/13000000.mali/cur_freq",
            "/sys/class/misc/mali0/device/devfreq/13000000.mali/cur_freq",
        ].iter().find_map(|path| fs::read_to_string(path).ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok()));
        let value = if ged > 0 { ged } else { devfreq.unwrap_or(0) };
        if value > 0 && value < 10_000_000 { value.saturating_mul(1000) }
        else if value > 0 && value < 100_000_000 { value.saturating_mul(10) }
        else { value }
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
