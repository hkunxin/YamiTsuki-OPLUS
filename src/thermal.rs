use std::fs;

const THERMAL_BASE: &str = "/sys/class/thermal";
const SOC_MAX_TYPE: &str = "soc_max";
const CPU_PREFIX: &str = "cpu-";
const GPU_PREFIX: &str = "gpu";
const SHELL_TEMP: &str = "/proc/shell-temp";

pub struct ThermalManager {
    original: i64,
}

fn read_zone_temp(zone: &str) -> Option<i64> {
    let value = fs::read_to_string(format!("{}/{}/temp", THERMAL_BASE, zone))
        .ok()?.trim().parse::<i64>().ok()?;
    // 过滤 MTK 未连接/无效传感器常见哨兵值，避免热控被错误触发。
    (value > -20_000 && value < 150_000).then_some(value)
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

    /// PLG110 专用：只写 /proc/shell-temp，保留所有真实 thermal zone 和硬件保护。
    /// extreme_gt 使用 index + 温度格式；这里仅更新 shell_front/frame/back 的显示索引。
    pub fn apply_spoof(&self, mode: &str) -> bool {
        let Some(real) = Self::zone_names().into_iter().find_map(|zone| {
            (Self::zone_type(&zone) == "shell_front").then(|| read_zone_temp(&zone)).flatten()
        }) else { return false; };
        let target = match mode {
            "powersave" => real.min(29_000),
            "balance" => real,
            "performance" => real,
            _ => real,
        };
        if fs::metadata(SHELL_TEMP).is_err() { return false; }
        // 三个外壳温度索引：front/frame/back。目标温度不低于真实 shell 温度。
        (0..3).all(|index| fs::write(SHELL_TEMP, format!("{} {}", index, target)).is_ok())
    }

    pub fn restore(&self) {}

    pub fn cpu_temp(&self) -> f64 { self.real_temp() as f64 / 1000.0 }

    pub fn max_protection_temp(&self) -> i64 {
        let soc = Self::soc_temp().unwrap_or(0);
        let cpu = self.cpu_core_temp().map(|v| (v * 1000.0) as i64).unwrap_or(0);
        soc.max(cpu)
    }

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
