use std::fs;

const THERMAL_BASE: &str = "/sys/class/thermal";
const SOC_MAX_TYPE: &str = "soc_max";
const CPU_PREFIX: &str = "cpu-";
const GPU_PREFIX: &str = "gpu";

pub struct ThermalManager {
    original: i64,
}

fn read_zone_temp(zone: &str) -> Option<i64> {
    fs::read_to_string(format!("{}/{}/temp", THERMAL_BASE, zone))
        .ok()?.trim().parse().ok()
}

impl ThermalManager {
    pub fn new() -> Self {
        ThermalManager { original: Self::soc_temp().unwrap_or(35_000) }
    }

    fn zone_names() -> Vec<String> {
        let Ok(entries) = fs::read_dir(THERMAL_BASE) else { return Vec::new(); };
        entries.filter_map(|entry| {
            let name = entry.ok()?.file_name().to_string_lossy().into_owned();
            name.strip_prefix("thermal_zone").and_then(|n| n.parse::<u32>().ok()).map(|n| format!("thermal_zone{}", n))
        }).collect()
    }

    fn zone_type(zone: &str) -> String {
        fs::read_to_string(format!("{}/{}/type", THERMAL_BASE, zone)).unwrap_or_default().trim().to_string()
    }

    fn soc_temp() -> Option<i64> {
        Self::zone_names().into_iter().find_map(|zone| {
            (Self::zone_type(&zone) == SOC_MAX_TYPE).then(|| read_zone_temp(&zone)).flatten()
        })
    }

    pub fn real_temp(&self) -> i64 { Self::soc_temp().unwrap_or(self.original) }

    pub fn spoof_temp(&self, mode: &str) -> i64 {
        match mode { "powersave" => 28_000, "balance" => 35_000, "performance" => 42_000, _ => 35_000 }
    }

    /// Thermal sensor temp nodes are normally read-only. Do not write arbitrary charger zones.
    /// Return false unless a vendor fake-temp node explicitly exists and accepts the value.
    pub fn apply_spoof(&self, mode: &str) -> bool {
        let target = self.spoof_temp(mode).to_string();
        ["/sys/kernel/oplus_thermal/fake_temp", "/sys/class/thermal/thermal_message/sconfig"]
            .iter().filter(|path| fs::metadata(path).is_ok()).any(|path| fs::write(path, &target).is_ok())
    }

    pub fn restore(&self) {}

    pub fn cpu_temp(&self) -> f64 { self.real_temp() as f64 / 1000.0 }

    pub fn gpu_temp(&self) -> Option<f64> {
        Self::zone_names().into_iter().filter_map(|zone| {
            let kind = Self::zone_type(&zone);
            (kind.starts_with(GPU_PREFIX)).then(|| read_zone_temp(&zone)).flatten()
        }).max().map(|v| v as f64 / 1000.0)
    }

    pub fn cpu_core_temp(&self) -> Option<f64> {
        Self::zone_names().into_iter().filter_map(|zone| {
            let kind = Self::zone_type(&zone);
            (kind.starts_with(CPU_PREFIX)).then(|| read_zone_temp(&zone)).flatten()
        }).max().map(|v| v as f64 / 1000.0)
    }
}
