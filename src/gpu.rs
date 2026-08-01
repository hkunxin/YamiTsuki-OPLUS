use std::fs;

const GPU_MAX_FREQ: &str = "/sys/class/kgsl/kgsl-3d0/max_gpuclk";
const GPU_AVAIL_FREQ: &str = "/sys/class/kgsl/kgsl-3d0/gpu_available_frequencies";
const GPU_FORCE_RAIL: &str = "/sys/class/kgsl/kgsl-3d0/force_rail_on";

// PLG110: gpu_freq is a thermal limit list, not the current clock.
const GED_GPU_FREQ: &str = "/sys/kernel/thermal/gpu_freq";
const GED_CURRENT_FREQ: &str = "/sys/kernel/ged/hal/current_freqency";
const GED_UTIL: &str = "/sys/kernel/ged/hal/gpu_utilization";
const GED_SUM_LOADING: &str = "/sys/kernel/ged/hal/gpu_sum_loading";
const GED_BOOST_FREQ: &str = "/sys/kernel/ged/hal/custom_boost_gpu_freq";
const GED_UPBOUND_FREQ: &str = "/sys/kernel/ged/hal/custom_upbound_gpu_freq";

pub struct GpuManager {
    vendor: GpuVendor,
    available_freqs: Vec<u64>,
}

#[derive(PartialEq)]
enum GpuVendor {
    Adreno,
    GedMali,
    Unknown,
}

fn numbers(raw: &str) -> Vec<u64> {
    raw.split_whitespace()
        .filter_map(|part| part.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok())
        .filter(|value: &u64| *value > 0)
        .collect()
}

fn readable(path: &str) -> bool {
    fs::read_to_string(path).map(|s| !s.trim().is_empty()).unwrap_or(false)
}

impl GpuManager {
    pub fn new() -> Self {
        if readable(GPU_MAX_FREQ) {
            let freqs = fs::read_to_string(GPU_AVAIL_FREQ)
                .map(|raw| numbers(&raw))
                .unwrap_or_default();
            GpuManager { vendor: GpuVendor::Adreno, available_freqs: freqs }
        } else if readable(GED_UTIL)
            || readable(GED_SUM_LOADING)
            || readable(GED_CURRENT_FREQ)
            || readable(GED_BOOST_FREQ)
            || readable(GED_UPBOUND_FREQ)
            || readable(GED_GPU_FREQ)
        {
            let mut freqs = Vec::new();
            for path in [GED_BOOST_FREQ, GED_UPBOUND_FREQ, GED_GPU_FREQ] {
                if let Ok(raw) = fs::read_to_string(path) {
                    freqs.extend(numbers(&raw));
                }
            }
            freqs.sort_unstable();
            freqs.dedup();
            GpuManager { vendor: GpuVendor::GedMali, available_freqs: freqs }
        } else {
            GpuManager { vendor: GpuVendor::Unknown, available_freqs: Vec::new() }
        }
    }

    pub fn current_freq(&self) -> u64 {
        match self.vendor {
            GpuVendor::Adreno => fs::read_to_string(GPU_MAX_FREQ)
                .ok().and_then(|raw| numbers(&raw).into_iter().next()).unwrap_or(0),
            GpuVendor::GedMali => fs::read_to_string(GED_CURRENT_FREQ)
                .ok().and_then(|raw| numbers(&raw).into_iter().next()).unwrap_or(0),
            GpuVendor::Unknown => 0,
        }
    }

    fn target_freq(&self, mode: &str) -> Option<u64> {
        if self.available_freqs.is_empty() { return None; }
        match mode {
            "performance" => self.available_freqs.last().copied(),
            "powersave" => self.available_freqs.first().copied(),
            _ => self.available_freqs.get(self.available_freqs.len() / 2).copied(),
        }
    }

    pub fn apply_mode(&self, mode: &str) {
        match self.vendor {
            GpuVendor::Adreno => {
                let Some(target) = self.target_freq(mode) else { return; };
                let _ = fs::write(GPU_MAX_FREQ, target.to_string());
                let _ = fs::write(GPU_FORCE_RAIL, if mode == "performance" { "1" } else { "0" });
            }
            GpuVendor::GedMali => {
                // PLG110 GED control units/ranges are not confirmed. Read-only for safety.
            }
            GpuVendor::Unknown => {}
        }
    }

    pub fn vendor_name(&self) -> &str {
        match self.vendor {
            GpuVendor::Adreno => "Adreno",
            GpuVendor::GedMali => "Mali (GED)",
            GpuVendor::Unknown => "Unknown",
        }
    }
}

pub(crate) const GED_UTIL_PATH: &str = GED_UTIL;
pub(crate) const GED_LOADING_PATH: &str = GED_SUM_LOADING;
pub(crate) const GED_CURRENT_FREQ_PATH: &str = GED_CURRENT_FREQ;
