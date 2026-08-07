use std::fs;
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

const MODE_FILE: &str = "/data/local/tmp/yamitsuki_mode";
const GAME_LIST: &str = "/data/adb/modules/yamitsuki_oplus/game_list.txt";
const BATTERY_CAPACITY: &str = "/sys/class/power_supply/battery/capacity";
const BATTERY_STATUS: &str = "/sys/class/power_supply/battery/status";
const SCREEN_POLL_SECS: u64 = 5;
const GAME_POLL_SECS: u64 = 5;

pub struct ModeManager {
    screen_off_cache: Mutex<Option<(Instant, bool)>>,
    game_running_cache: Mutex<Option<(Instant, bool)>>,
}

impl ModeManager {
    pub fn new() -> Self {
        ModeManager {
            screen_off_cache: Mutex::new(None),
            game_running_cache: Mutex::new(None),
        }
    }

    pub fn active_mode(&self) -> String {
        let mode = fs::read_to_string(MODE_FILE).unwrap_or_else(|_| "auto".to_string()).trim().to_string();
        match mode.as_str() {
            "powersave" | "balance" | "performance" => mode,
            "auto" => self.auto_mode(),
            _ => "balance".to_string(),
        }
    }

    pub fn decision_reason(&self) -> String {
        let mode = fs::read_to_string(MODE_FILE).unwrap_or_else(|_| "auto".to_string()).trim().to_string();
        match mode.as_str() {
            "powersave" | "balance" | "performance" => format!("显式模式: {}", mode),
            "auto" => self.auto_decision().1,
            _ => format!("未知模式 {:?}，回退 balance", mode),
        }
    }

    fn auto_mode(&self) -> String { self.auto_decision().0 }

    fn auto_decision(&self) -> (String, String) {
        let screen_off = self.is_screen_off();
        let battery = self.battery_level();
        let charging = self.is_charging();
        if screen_off {
            ("powersave".to_string(), "自动: 屏幕关闭".to_string())
        } else if battery < 15 && !charging {
            ("powersave".to_string(), format!("自动: 电量低于 15% 且未充电 ({}%)", battery))
        } else if self.is_game_running() && (battery > 25 || charging) {
            ("performance".to_string(), if charging {
                format!("自动: 游戏运行且正在充电 ({}%)", battery)
            } else {
                format!("自动: 游戏运行且电量充足 ({}%)", battery)
            })
        } else if charging {
            ("balance".to_string(), format!("自动: 正在充电 ({}%)", battery))
        } else {
            ("balance".to_string(), format!("自动: 常规状态 ({}%)", battery))
        }
    }

    fn battery_level(&self) -> u32 { fs::read_to_string(BATTERY_CAPACITY).unwrap_or_default().trim().parse().unwrap_or(100) }
    fn is_charging(&self) -> bool { matches!(fs::read_to_string(BATTERY_STATUS).unwrap_or_default().trim(), "Charging" | "Full") }

    pub fn is_screen_off(&self) -> bool {
        if let Some((at, cached)) = *self.screen_off_cache.lock().unwrap() {
            if at.elapsed().as_secs() < SCREEN_POLL_SECS {
                return cached;
            }
        }
        let result = Self::probe_screen_off();
        *self.screen_off_cache.lock().unwrap() = Some((Instant::now(), result));
        result
    }

    fn probe_screen_off() -> bool {
        let Ok(output) = Command::new("dumpsys").arg("display").output() else { return false; };
        let text = String::from_utf8_lossy(&output.stdout);
        if text.contains("mScreenState=ON") || text.contains("state=ON") { return false; }
        text.contains("mScreenState=OFF") || text.contains("state=OFF") || text.contains("state=DOZE")
    }

    fn is_game_running(&self) -> bool {
        if let Some((at, cached)) = *self.game_running_cache.lock().unwrap() {
            if at.elapsed().as_secs() < GAME_POLL_SECS {
                return cached;
            }
        }
        let result = Self::probe_game_running();
        *self.game_running_cache.lock().unwrap() = Some((Instant::now(), result));
        result
    }

    fn probe_game_running() -> bool {
        let Ok(list) = fs::read_to_string(GAME_LIST) else { return false; };
        let Ok(output) = Command::new("dumpsys").args(["activity", "activities"]).output() else { return false; };
        let activity = String::from_utf8_lossy(&output.stdout);
        list.lines()
            .map(|line| line.split('#').next().unwrap_or("").trim())
            .filter(|pkg| !pkg.is_empty())
            .any(|pkg| activity.contains(pkg))
    }

    #[allow(dead_code)]
    fn _is_charging(&self) -> bool { self.is_charging() }
}
