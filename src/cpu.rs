use std::fs;
use std::sync::Mutex;

const CPU_BASE: &str = "/sys/devices/system/cpu";
const CPU_POSSIBLE: &str = "/sys/devices/system/cpu/possible";

pub struct CpuManager {
    cores: Vec<u32>,
    little_max: usize,
    stat_prev: Mutex<Option<(u64, u64)>>,
    pub big_cores: Vec<u32>,
    pub little_cores: Vec<u32>,
    pub middle_cores: Vec<u32>,
    pub prime_cores: Vec<u32>,
}

fn parse_cpu_range(raw: &str) -> Vec<u32> {
    let mut cores = Vec::new();
    for part in raw.trim().split(',') {
        let mut bounds = part.trim().split('-');
        let Some(start) = bounds.next().and_then(|value| value.parse::<u32>().ok()) else { continue; };
        let end = bounds.next().and_then(|value| value.parse::<u32>().ok()).unwrap_or(start);
        for core in start..=end { cores.push(core); }
    }
    cores.sort_unstable();
    cores.dedup();
    cores
}

fn read_cpu_package(core: u32) -> Option<u32> {
    fs::read_to_string(format!("{}/cpu{}/topology/physical_package_id", CPU_BASE, core))
        .ok()?.trim().parse::<u32>().ok()
}

fn read_cpu_capacity(core: u32) -> Option<u64> {
    let candidates = [
        format!("{}/cpu{}/cpu_capacity", CPU_BASE, core),
        format!("{}/cpu{}/cpu_capacity_orig", CPU_BASE, core),
        format!("{}/cpu{}/capacity", CPU_BASE, core),
        format!("{}/cpu{}/topology/cpu_capacity", CPU_BASE, core),
    ];
    candidates.into_iter().find_map(|path| {
        fs::read_to_string(path).ok()?.trim().parse::<u64>().ok().filter(|value| *value > 0)
    })
}

impl CpuManager {
    pub fn new() -> Self {
        let mut big = vec![];
        let mut little = vec![];
        let mut middle = vec![];
        let mut prime = vec![];
        let mut possible = parse_cpu_range(&fs::read_to_string(CPU_POSSIBLE).unwrap_or_default());
        if possible.is_empty() {
            possible = fs::read_dir(CPU_BASE)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                .filter_map(|name| name.strip_prefix("cpu").and_then(|value| value.parse::<u32>().ok()))
                .collect();
            possible.sort_unstable();
        }
        let mut cores = Vec::new();
        let mut capacities = Vec::new();

        for i in possible {
            let path = format!("{}/cpu{}/cpufreq/cpuinfo_max_freq", CPU_BASE, i);
            if let Ok(freq_str) = fs::read_to_string(&path) {
                cores.push(i);
                let max_freq: u64 = freq_str.trim().parse().unwrap_or(0);
                capacities.push((i, read_cpu_package(i), read_cpu_capacity(i), max_freq));
            }
        }

        let capacity_values: Vec<u64> = capacities.iter().filter_map(|(_, _, capacity, _)| *capacity).collect();
        if !capacity_values.is_empty() {
            let min_capacity = *capacity_values.iter().min().unwrap_or(&0);
            let max_capacity = *capacity_values.iter().max().unwrap_or(&0);
            let middle_capacity = capacity_values.iter()
                .copied()
                .filter(|value| *value > min_capacity && *value < max_capacity)
                .min();
            for (core, _package, capacity, _max_freq) in capacities {
                let value = capacity.unwrap_or(min_capacity);
                if value == max_capacity {
                    prime.push(core);
                    big.push(core);
                } else if middle_capacity.is_some_and(|middle_value| value == middle_value) {
                    middle.push(core);
                } else {
                    little.push(core);
                }
            }
        } else {
            for (core, _package, _capacity, max_freq) in capacities {
                if max_freq > 2_000_000 {
                    prime.push(core);
                    big.push(core);
                } else {
                    little.push(core);
                }
            }
        }

        let little_max = little.len();

        CpuManager {
            cores,
            little_max,
            stat_prev: Mutex::new(None),
            big_cores: big,
            little_cores: little,
            middle_cores: middle,
            prime_cores: prime,
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

    pub fn last_core(&self) -> Option<u32> {
        self.cores.last().copied()
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
    pub fn apply_dynamic_cap(&self, mode: &str, cpu_load: u32, temp_mc: i64, power_watts: f64) -> u32 {
        let (base, max_with_load): (f64, f64) = match mode {
            "powersave" => (0.40, 0.50),
            "performance" => (0.85, 1.00),
            _ => (0.65, 0.85),
        };
        let demand_boost: f64 = if cpu_load >= 85 && power_watts < 3.5 { 0.20 }
            else if cpu_load >= 65 && power_watts < 3.0 { 0.10 }
            else if cpu_load >= 45 && power_watts < 2.5 { 0.05 }
            else { 0.0 };
        let power_limit: f64 = if power_watts >= 5.0 { 0.70 }
            else if power_watts >= 4.0 { 0.85 }
            else { 1.0 };
        let thermal_limit: f64 = if temp_mc >= 56_000 { 0.45 }
            else if temp_mc >= 52_000 { 0.65 }
            else if temp_mc >= 48_000 { 0.85 }
            else { 1.0 };
        let factor = (base + demand_boost).min(max_with_load) * thermal_limit * power_limit;

        for (policy, related) in self.policy_groups() {
            if let Some(&core) = related.first() {
                self.write_policy_max(&policy, core, factor);
            }
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
        let Some(&core) = self.cores.first() else { return Vec::new(); };
        let path = format!("{}/cpu{}/cpufreq/scaling_available_governors", CPU_BASE, core);
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn current_governor(&self) -> String {
        let Some(&core) = self.cores.first() else { return String::new(); };
        let path = format!("{}/cpu{}/cpufreq/scaling_governor", CPU_BASE, core);
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .to_string()
    }

    fn policy_groups(&self) -> Vec<(String, Vec<u32>)> {
        let policy_base = format!("{}/cpufreq", CPU_BASE);
        let mut groups = Vec::new();
        if let Ok(entries) = fs::read_dir(&policy_base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !matches!(name.as_str(), "policy0" | "policy4" | "policy7") {
                    continue;
                }
                let path = entry.path();
                let related = fs::read_to_string(path.join("related_cpus"))
                    .map(|raw| parse_cpu_range(&raw))
                    .unwrap_or_default();
                if !related.is_empty() {
                    groups.push((path.to_string_lossy().to_string(), related));
                }
            }
        }
        groups
    }

    fn write_policy_max(&self, policy: &str, core: u32, factor: f64) {
        let max_path = format!("{}/cpuinfo_max_freq", policy);
        let min_path = format!("{}/cpuinfo_min_freq", policy);
        let target_path = format!("{}/scaling_max_freq", policy);
        let max = fs::read_to_string(&max_path)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or_else(|| self.read_max_freq(core));
        let min = fs::read_to_string(&min_path)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or_else(|| self.read_min_freq(core));
        if max > 0 {
            let _ = fs::write(target_path, ((max as f64 * factor) as u64).max(min).to_string());
        }
    }

    pub fn set_all_governors(&self, gov: &str) {
        for (policy, _) in self.policy_groups() {
            let path = format!("{}/scaling_governor", policy);
            if std::path::Path::new(&path).exists() {
                let _ = fs::write(path, gov);
            }
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

        let factors = match mode {
            "powersave" => (0.4, 0.45, 0.5),
            "balance" => (0.8, 0.75, 0.7),
            "performance" => (1.0, 1.0, 1.0),
            _ => return,
        };
        for (policy, related) in self.policy_groups() {
            let factor = if related.iter().any(|core| self.little_cores.contains(core)) {
                factors.0
            } else if related.iter().any(|core| self.middle_cores.contains(core)) {
                factors.1
            } else {
                factors.2
            };
            if let Some(&core) = related.first() {
                self.write_policy_max(&policy, core, factor);
            }
        }
    }
}
