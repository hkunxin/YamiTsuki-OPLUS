use std::fs;

// Adreno (Snapdragon)
const GPU_MAX_FREQ: &str = "/sys/class/kgsl/kgsl-3d0/max_gpuclk";
const GPU_AVAIL_FREQ: &str = "/sys/class/kgsl/kgsl-3d0/gpu_available_frequencies";
const GPU_FORCE_RAIL: &str = "/sys/class/kgsl/kgsl-3d0/force_rail_on";

// Mali (MediaTek Dimensity)
const MALI_MAX_FREQ: &str = "/sys/class/misc/mali0/device/max_freq";
const MALI_AVAIL_FREQ: &str = "/sys/class/misc/mali0/device/available_frequencies";
const MALI_CUR_FREQ: &str = "/sys/class/misc/mali0/device/cur_freq";

/// Unified GPU manager supporting both Adreno (Snapdragon) and Mali (Dimensity)
pub struct GpuManager {
    vendor: GpuVendor,
    available_freqs: Vec<u64>,
}

#[derive(PartialEq)]
enum GpuVendor {
    Adreno,
    Mali,
    Unknown,
}

impl GpuManager {
    pub fn new() -> Self {
        // Auto-detect GPU vendor
        if fs::metadata(GPU_MAX_FREQ).is_ok() {
            let freqs = fs::read_to_string(GPU_AVAIL_FREQ)
                .unwrap_or_default()
                .trim()
                .split_whitespace()
                .filter_map(|s| s.parse::<u64>().ok())
                .collect();
            GpuManager {
                vendor: GpuVendor::Adreno,
                available_freqs: freqs,
            }
        } else if fs::metadata(MALI_MAX_FREQ).is_ok() || fs::metadata(MALI_AVAIL_FREQ).is_ok() {
            let freqs = fs::read_to_string(MALI_AVAIL_FREQ)
                .unwrap_or_default()
                .trim()
                .split_whitespace()
                .filter_map(|s| s.parse::<u64>().ok())
                .collect();
            GpuManager {
                vendor: GpuVendor::Mali,
                available_freqs: freqs,
            }
        } else {
            GpuManager {
                vendor: GpuVendor::Unknown,
                available_freqs: vec![],
            }
        }
    }

    pub fn current_freq(&self) -> u64 {
        match self.vendor {
            GpuVendor::Adreno => {
                fs::read_to_string(GPU_MAX_FREQ)
                    .unwrap_or_default()
                    .trim()
                    .parse()
                    .unwrap_or(0)
            }
            GpuVendor::Mali => {
                fs::read_to_string(MALI_CUR_FREQ)
                    .unwrap_or_default()
                    .trim()
                    .parse()
                    .unwrap_or(0)
            }
            GpuVendor::Unknown => 0,
        }
    }

    fn max_gpu_freq(&self) -> u64 {
        self.available_freqs.last().copied().unwrap_or(700_000_000)
    }

    fn mid_gpu_freq(&self) -> u64 {
        if self.available_freqs.len() >= 2 {
            let mid_idx = self.available_freqs.len() / 2;
            self.available_freqs[mid_idx]
        } else {
            self.max_gpu_freq() / 2
        }
    }

    fn min_gpu_freq(&self) -> u64 {
        self.available_freqs.first().copied().unwrap_or(300_000_000)
    }

    pub fn apply_mode(&self, mode: &str) {
        let target = match mode {
            "performance" => self.max_gpu_freq(),
            "powersave" => self.min_gpu_freq(),
            _ => self.mid_gpu_freq(),
        };

        match self.vendor {
            GpuVendor::Adreno => {
                let _ = fs::write(GPU_MAX_FREQ, target.to_string());
                if mode == "performance" {
                    let _ = fs::write(GPU_FORCE_RAIL, "1");
                } else {
                    let _ = fs::write(GPU_FORCE_RAIL, "0");
                }
            }
            GpuVendor::Mali => {
                let _ = fs::write(MALI_MAX_FREQ, target.to_string());
            }
            GpuVendor::Unknown => {}
        }
    }

    pub fn vendor_name(&self) -> &str {
        match self.vendor {
            GpuVendor::Adreno => "Adreno",
            GpuVendor::Mali => "Mali",
            GpuVendor::Unknown => "Unknown",
        }
    }
}
