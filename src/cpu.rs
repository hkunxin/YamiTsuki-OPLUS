use std::fs;
use std::sync::Mutex;

const CPU_BASE: &str = "/sys/devices/system/cpu";
const CPU_MAX: u32 = 8;

pub struct CpuManager {
    cores: u32,
    little_max: usize,
    stat_prev: Mutex<Option<(u64, u64)>>,
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
            stat_prev: Mutex::new(None),
            big_cores: big,
            little_cores: little,
        }
    }

    /// 通过两次 /proc/stat 差值计算真实 CPU 利用率，避免把 loadavg 当成百分比。
    /// 首次采样或计数器异常时返回 0，避免省电模式误放宽频率上限。
    pub fn load_percent(&self) -> u32 {
        let line = fs::read_to_string("/proc/stat").ok()
            .and_then(|raw| raw.lines().find(|line| line.starts_with("cpu ")).map(str::to_string));
        let values: Vec<u64> = line.unwrap_or_default().split_whitespace().skip(1)
            .filter_map(|v| v.parse().ok()).collect();
        if values.len() < 4 { return 0; }
        let idle = values[3].saturating_add(*values.get(4).unwrap_or(&0));
        let total = values.iter().fold(0u64, |sum, value| sum.saturating_add(*value));
        let mut previous = self.stat_prev.lock().unwrap();
        let result = previous.map(|(old_total, old_idle)| {
            let total_delta = total.saturating_sub(old_total);
            let idle_delta = idle.saturating_sub(old_idle);
            if total_delta == 0 { 0 } else {
                ((total_delta.saturating_sub(idle_delta) * 100) / total_delta).min(100) as u32
            }
        }).unwrap_or(0);
        *previous = Some((total, idle));
        result
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
