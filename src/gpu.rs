use std::fs;
use std::path::Path;

const GPU_MAX_FREQ: &str = "/sys/class/kgsl/kgsl-3d0/max_gpuclk";
const GPU_AVAIL_FREQ: &str = "/sys/class/kgsl/kgsl-3d0/gpu_available_frequencies";
const GPU_FORCE_RAIL: &str = "/sys/class/kgsl/kgsl-3d0/force_rail_on";

const DEVFREQ_ROOT: &str = "/sys/devices/platform/soc/13000000.mali/devfreq/13000000.mali";
const GED_CURRENT_FREQ: &str = "/sys/kernel/ged/hal/current_freqency";
const GED_UTIL: &str = "/sys/kernel/ged/hal/gpu_utilization";
const GED_SUM_LOADING: &str = "/sys/kernel/ged/hal/gpu_sum_loading";
const GED_BOOST_FREQ: &str = "/sys/kernel/ged/hal/custom_boost_gpu_freq";
const GED_UPBOUND_FREQ: &str = "/sys/kernel/ged/hal/custom_upbound_gpu_freq";

pub struct GpuManager {
    vendor: GpuVendor,
    available_freqs: Vec<u64>,
    devfreq_root: Option<String>,
}

#[derive(PartialEq)]
enum GpuVendor { Adreno, DevfreqMali, GedMali, Unknown }

fn numbers(raw: &str) -> Vec<u64> {
    raw.split_whitespace()
        .filter_map(|part| part.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok())
        .filter(|value: &u64| *value > 0)
        .collect()
}

fn readable(path: &str) -> bool { fs::read_to_string(path).map(|s| !s.trim().is_empty()).unwrap_or(false) }

fn find_devfreq() -> Option<String> {
    let candidates = [DEVFREQ_ROOT.to_string(), "/sys/devices/platform/soc/13000000.mali/devfreq/".to_string()];
    if Path::new(DEVFREQ_ROOT).join("max_freq").exists() { return Some(DEVFREQ_ROOT.to_string()); }
    for base in candidates.iter().skip(1) {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.join("max_freq").exists() { return Some(path.to_string_lossy().into_owned()); }
            }
        }
    }
    None
}

fn read_freq_list(root: &str) -> Vec<u64> {
    fs::read_to_string(Path::new(root).join("available_frequencies"))
        .map(|raw| { let mut v = numbers(&raw); v.sort_unstable(); v.dedup(); v }).unwrap_or_default()
}

impl GpuManager {
    pub fn new() -> Self {
        if readable(GPU_MAX_FREQ) {
            let freqs = fs::read_to_string(GPU_AVAIL_FREQ).map(|raw| numbers(&raw)).unwrap_or_default();
            GpuManager { vendor: GpuVendor::Adreno, available_freqs: freqs, devfreq_root: None }
        } else if let Some(root) = find_devfreq() {
            let freqs = read_freq_list(&root);
            GpuManager { vendor: GpuVendor::DevfreqMali, available_freqs: freqs, devfreq_root: Some(root) }
        } else if readable(GED_UTIL) || readable(GED_SUM_LOADING) || readable(GED_CURRENT_FREQ) || readable(GED_BOOST_FREQ) || readable(GED_UPBOUND_FREQ) {
            GpuManager { vendor: GpuVendor::GedMali, available_freqs: Vec::new(), devfreq_root: None }
        } else {
            GpuManager { vendor: GpuVendor::Unknown, available_freqs: Vec::new(), devfreq_root: None }
        }
    }

    pub fn current_freq(&self) -> u64 {
        match self.vendor {
            GpuVendor::Adreno => fs::read_to_string(GPU_MAX_FREQ).ok().and_then(|raw| numbers(&raw).into_iter().next()).unwrap_or(0),
            GpuVendor::DevfreqMali => self.devfreq_root.as_ref().and_then(|root| fs::read_to_string(Path::new(root).join("cur_freq")).ok()).and_then(|raw| numbers(&raw).into_iter().next()).unwrap_or(0),
            GpuVendor::GedMali => fs::read_to_string(GED_CURRENT_FREQ).ok().and_then(|raw| numbers(&raw).nth(1)).unwrap_or(0),
            GpuVendor::Unknown => 0,
        }
    }

    fn target_freq(&self, mode: &str) -> Option<u64> {
        if self.available_freqs.is_empty() { return None; }
        match mode {
            "performance" => self.available_freqs.last().copied(),
            "powersave" => self.available_freqs.first().copied(),
            _ => self.available_freqs.iter().rev().copied().find(|f| *f <= 780_000_000).or_else(|| self.available_freqs.first().copied()),
        }
    }

    pub fn apply_mode(&self, mode: &str) {
        match self.vendor {
            GpuVendor::Adreno => {
                if let Some(target) = self.target_freq(mode) { let _ = fs::write(GPU_MAX_FREQ, target.to_string()); }
                let _ = fs::write(GPU_FORCE_RAIL, if mode == "performance" { "1" } else { "0" });
            }
            GpuVendor::DevfreqMali => {
                let Some(root) = &self.devfreq_root else { return; };
                if let Some(target) = self.target_freq(mode) { let _ = fs::write(Path::new(root).join("max_freq"), target.to_string()); }
                let governor = match mode { "powersave" => "powersave", "performance" => "performance", _ => "simple_ondemand" };
                let gov_path = Path::new(root).join("governor");
                if fs::read_to_string(Path::new(root).join("available_governors")).map(|s| s.split_whitespace().any(|g| g == governor)).unwrap_or(false) { let _ = fs::write(gov_path, governor); }
            }
            GpuVendor::GedMali | GpuVendor::Unknown => {}
        }
    }

    pub fn vendor_name(&self) -> &str {
        match self.vendor { GpuVendor::Adreno => "Adreno", GpuVendor::DevfreqMali => "Mali (devfreq)", GpuVendor::GedMali => "Mali (GED)", GpuVendor::Unknown => "Unknown" }
    }
}

pub(crate) const GED_UTIL_PATH: &str = GED_UTIL;
pub(crate) const GED_LOADING_PATH: &str = GED_SUM_LOADING;
pub(crate) const GED_CURRENT_FREQ_PATH: &str = GED_CURRENT_FREQ;
