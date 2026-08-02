use std::fs;

const CPU_BASE: &str = "/sys/devices/system/cpu";
const CPU_MAX: u32 = 8;

pub struct CpuManager {
    cores: u32,
    little_max: usize,
    pub big_cores: Vec<u32>,
    pub little_cores: Vec<u32>,
}

impl CpuManager {
    pub fn new() -> Self {
        let mut big = vec![];
        let mut little = vec![];
        let mut max_core = 0u32;

        for i in 0..CPU_MAX {
            let path = format!("{}/cpu{}/cpufreq/cpuinfo_max_freq", CPU_BASE, i);
            if let Ok(freq_str) = fs::read_to_string(&path) {
                max_core = i + 1;
                let max_freq: u64 = freq_str.trim().parse().unwrap_or(0);
                // OPLUS: big cores typically > 2GHz
                if max_freq > 2_000_000 {
                    big.push(i);
                } else {
                    little.push(i);
                }
            } else {
                break;
            }
        }

        let little_max = little.len();

        CpuManager {
            cores: max_core,
            little_max,
            big_cores: big,
            little_cores: little,
        }
    }

    /// Android 上保持依赖最少：使用 loadavg 估算全局 CPU 压力。
    /// 该值不是每核精确利用率，但足够用于频率上限的迟滞式调节。
    pub fn load_percent(&self) -> u32 {
        let load = fs::read_to_string("/proc/loadavg")
            .ok()
            .and_then(|raw| raw.split_whitespace().next()?.parse::<f64>().ok())
            .unwrap_or(0.0);
        ((load / self.cores.max(1) as f64) * 100.0).round().clamp(0.0, 100.0) as u32
    }

    pub fn read_freq(&self, core: u32) -> u64 {
        let path = format!("{}/cpu{}/cpufreq/scaling_cur_freq", CPU_BASE, core);
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(0)
    }

    pub fn read_max_freq(&self, core: u32) -> u64 {
        let path = format!("{}/cpu{}/cpufreq/cpuinfo_max_freq", CPU_BASE, core);
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(0)
    }

    pub fn read_min_freq(&self, core: u32) -> u64 {
        let path = format!("{}/cpu{}/cpufreq/cpuinfo_min_freq", CPU_BASE, core);
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(0)
    }

    pub fn set_scaling_max(&self, core: u32, freq: u64) -> bool {
        let path = format!("{}/cpu{}/cpufreq/scaling_max_freq", CPU_BASE, core);
        fs::write(&path, freq.to_string()).is_ok()
    }

    /// PLG110 保守动态上限：仅采用 CPU 压力，不让未校准的 GED GPU 值触发提频。
    /// 返回实际频率上限系数的千分比，供诊断日志使用。
    pub fn apply_dynamic_cap(&self, mode: &str, cpu_load: u32, temp_mc: i64) -> u32 {
        let (base, max_with_load): (f64, f64) = match mode {
            "powersave" => (0.40, 0.50),
            "performance" => (0.85, 1.00),
            _ => (0.65, 0.85),
        };
        let demand_boost: f64 = if cpu_load >= 85 { 0.20 }
            else if cpu_load >= 65 { 0.10 }
            else if cpu_load >= 45 { 0.05 }
            else { 0.0 };
        let thermal_limit: f64 = if temp_mc >= 56_000 { 0.45 }
            else if temp_mc >= 52_000 { 0.65 }
            else if temp_mc >= 48_000 { 0.85 }
            else { 1.0 };
        let factor = (base + demand_boost).min(max_with_load) * thermal_limit;

        for core in 0..self.cores {
            let hw_max = self.read_max_freq(core);
            let min = self.read_min_freq(core);
            if hw_max == 0 { continue; }
            let target = ((hw_max as f64 * factor) as u64).max(min);
            let _ = self.set_scaling_max(core, target);
        }
        (factor * 1000.0).round() as u32
    }

    fn write_max_freq(&self, core: u32, freq: u64) -> bool {
        self.set_scaling_max(core, freq)
    }

    fn write_min_freq(&self, core: u32, freq: u64) -> bool {
        let path = format!("{}/cpu{}/cpufreq/scaling_min_freq", CPU_BASE, core);
        fs::write(&path, freq.to_string()).is_ok()
    }

    pub fn available_governors(&self) -> Vec<String> {
        let path = format!("{}/cpu0/cpufreq/scaling_available_governors", CPU_BASE);
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn current_governor(&self) -> String {
        let path = format!("{}/cpu0/cpufreq/scaling_governor", CPU_BASE);
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .to_string()
    }

    pub fn set_all_governors(&self, gov: &str) {
        for i in 0..self.cores {
            let path = format!("{}/cpu{}/cpufreq/scaling_governor", CPU_BASE, i);
            let _ = fs::write(&path, gov);
        }
    }

    pub fn governor_for_mode(&self, mode: &str) -> String {
        match mode {
            "performance" => "performance".to_string(),
            _ => "schedutil".to_string(),
        }
    }

    pub fn apply_mode(&self, mode: &str) {
        let saved_gov = fs::read_to_string(
            "/data/adb/modules/yamitsuki_oplus/governor_selected",
        )
        .unwrap_or_default()
        .trim()
        .to_string();

        let gov = if saved_gov.is_empty() || saved_gov == "auto" {
            self.governor_for_mode(mode)
        } else {
            saved_gov
        };
        self.set_all_governors(&gov);

        match mode {
            "powersave" => {
                // Little cores @ 40%, big cores @ 50%
                for &c in &self.little_cores {
                    let max = self.read_max_freq(c);
                    let target = (max as f64 * 0.4) as u64;
                    self.write_max_freq(c, target.max(self.read_min_freq(c)));
                }
                for &c in &self.big_cores {
                    let max = self.read_max_freq(c);
                    let target = (max as f64 * 0.5) as u64;
                    self.write_max_freq(c, target.max(self.read_min_freq(c)));
                }
            }
            "balance" => {
                // Little cores @ 80%, big cores @ 70%
                for &c in &self.little_cores {
                    let max = self.read_max_freq(c);
                    let target = (max as f64 * 0.8) as u64;
                    self.write_max_freq(c, target.max(self.read_min_freq(c)));
                }
                for &c in &self.big_cores {
                    let max = self.read_max_freq(c);
                    let target = (max as f64 * 0.7) as u64;
                    self.write_max_freq(c, target.max(self.read_min_freq(c)));
                }
            }
            "performance" => {
                // All cores @ 100%
                for i in 0..self.cores {
                    let max = self.read_max_freq(i);
                    self.write_max_freq(i, max);
                }
            }
            _ => {}
        }
    }
}
