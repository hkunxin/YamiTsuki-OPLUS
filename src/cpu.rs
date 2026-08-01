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
