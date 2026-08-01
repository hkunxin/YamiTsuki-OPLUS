use std::fs;
use std::time::Duration;

const GPU_BUSY: &str = "/sys/class/kgsl/kgsl-3d0/gpubusy";
const GPU_UTIL: &str = "/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage";

pub struct FasScheduler {
    history: [u32; 4],
    idx: usize,
}

impl FasScheduler {
    pub fn new() -> Self {
        FasScheduler {
            history: [0; 4],
            idx: 0,
        }
    }

    /// Read GPU utilization percentage.
    /// On Adreno: kgsl-3d0/gpubusy (format: "busy total")
    /// On Mali: try misc/mali0 device
    pub fn gpu_util(&self) -> u32 {
        // Try Adreno
        if let Ok(raw) = fs::read_to_string(GPU_BUSY) {
            let parts: Vec<&str> = raw.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                let busy: u64 = parts[0].parse().unwrap_or(0);
                let total: u64 = parts[1].parse().unwrap_or(1);
                if total > 0 {
                    return ((busy * 100) / total) as u32;
                }
            }
        }
        // Try Mali
        if let Ok(raw) = fs::read_to_string(
            "/sys/class/misc/mali0/device/gpu_busy_percentage",
        ) {
            return raw.trim().parse().unwrap_or(0);
        }
        // Try new Adreno
        if let Ok(raw) = fs::read_to_string(GPU_UTIL) {
            return raw.trim().parse().unwrap_or(0);
        }
        0
    }

    /// Push latest reading, return smoothed value
    pub fn update(&mut self) -> u32 {
        let curr = self.gpu_util();
        self.history[self.idx] = curr;
        self.idx = (self.idx + 1) % 4;
        let sum: u32 = self.history.iter().sum();
        sum / 4
    }

    /// Returns a frequency scale factor (0.0 .. 1.0) based on GPU load.
    /// High GPU load → return close to 1.0 (allow max freq)
    /// Low GPU load → return lower scale
    pub fn freq_scale(&self, avg_util: u32) -> f64 {
        if avg_util > 80 {
            1.0  // Near max — frame drops likely, boost CPU
        } else if avg_util > 60 {
            0.85
        } else if avg_util > 40 {
            0.7
        } else {
            0.5
        }
    }
}
