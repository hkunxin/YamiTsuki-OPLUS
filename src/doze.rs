use std::fs;
use std::process::Command;

/// Doze 深度休眠模块
/// - 息屏时自动进入深度休眠（冻结后台进程、限制网络/传感器）
/// - 亮屏时自动恢复
pub struct DozeManager {
    enabled: bool,
}

impl DozeManager {
    pub fn new() -> Self {
        DozeManager { enabled: false }
    }

    /// Enter deep doze: freeze background apps, enable aggressive idle
    pub fn enter_doze(&mut self) {
        if self.enabled {
            return;
        }
        self.enabled = true;

        // 1. Set device idle mode
        let _ = Command::new("dumpsys")
            .args(&["deviceidle", "force-idle", "deep"])
            .output();

        // 2. Kill unnecessary wakelocks
        self.release_wakelocks();

        // 3. Lower max CPU freq for background
        for cpu in &[4, 5, 6, 7] {
            // big cores
            let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_max_freq", cpu);
            let min = format!("/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_min_freq", cpu);
            if let Ok(min_freq) = fs::read_to_string(&min) {
                let _ = fs::write(&path, min_freq.trim());
            }
        }

        // 4. Disable animations
        let _ = Command::new("settings")
            .args(&["put", "global", "window_animation_scale", "0.0"])
            .output();
        let _ = Command::new("settings")
            .args(&["put", "global", "transition_animation_scale", "0.0"])
            .output();
        let _ = Command::new("settings")
            .args(&["put", "global", "animator_duration_scale", "0.0"])
            .output();

        // 5. Drop caches to reduce background activity
        let _ = fs::write("/proc/sys/vm/drop_caches", "3");
    }

    /// Exit doze: restore to normal
    pub fn exit_doze(&mut self) {
        if !self.enabled {
            return;
        }
        self.enabled = false;

        // 1. Exit device idle
        let _ = Command::new("dumpsys")
            .args(&["deviceidle", "unforce"])
            .output();

        // 2. Restore animations
        let _ = Command::new("settings")
            .args(&["put", "global", "window_animation_scale", "1.0"])
            .output();
        let _ = Command::new("settings")
            .args(&["put", "global", "transition_animation_scale", "1.0"])
            .output();
        let _ = Command::new("settings")
            .args(&["put", "global", "animator_duration_scale", "1.0"])
            .output();

        // 3. Restore big core frequencies (handled by main loop)
    }

    pub fn is_dozing(&self) -> bool {
        self.enabled
    }

    fn release_wakelocks(&self) {
        // Kill common wakelock holders
        let _ = Command::new("sh")
            .arg("-c")
            .arg("dumpsys battery unplug 2>/dev/null; sleep 0.5; dumpsys battery reset 2>/dev/null")
            .output();
    }
}
