use std::fs;
use std::process::Command;

const MODE_FILE: &str = "/data/local/tmp/yamitsuki_mode";
const GAME_LIST: &str = "/data/adb/modules/yamitsuki_oplus/game_list.txt";
const BATTERY_CAPACITY: &str = "/sys/class/power_supply/battery/capacity";
const BATTERY_STATUS: &str = "/sys/class/power_supply/battery/status";

pub struct ModeManager;

impl ModeManager {
    pub fn new() -> Self { ModeManager }

    pub fn active_mode(&self) -> String {
        let mode = fs::read_to_string(MODE_FILE).unwrap_or_else(|_| "auto".to_string()).trim().to_string();
        if mode != "auto" { return mode; }
        if self.is_game_running() { "performance".to_string() }
        else if self.is_screen_off() || self.battery_level() < 15 { "powersave".to_string() }
        else { "balance".to_string() }
    }

    fn battery_level(&self) -> u32 { fs::read_to_string(BATTERY_CAPACITY).unwrap_or_default().trim().parse().unwrap_or(100) }
    fn is_charging(&self) -> bool { matches!(fs::read_to_string(BATTERY_STATUS).unwrap_or_default().trim(), "Charging" | "Full") }

    pub fn is_screen_off(&self) -> bool {
        let Ok(output) = Command::new("dumpsys").arg("display").output() else { return false; };
        let text = String::from_utf8_lossy(&output.stdout);
        if text.contains("mScreenState=ON") || text.contains("state=ON") { return false; }
        text.contains("mScreenState=OFF") || text.contains("state=OFF") || text.contains("state=DOZE")
    }

    fn is_game_running(&self) -> bool {
        let Ok(list) = fs::read_to_string(GAME_LIST) else { return false; };
        list.lines().map(|line| line.split('#').next().unwrap_or("").trim()).filter(|pkg| !pkg.is_empty()).any(|pkg| {
            Command::new("pidof").arg(pkg).output().map(|output| !output.stdout.is_empty()).unwrap_or(false)
        })
    }

    #[allow(dead_code)]
    fn _is_charging(&self) -> bool { self.is_charging() }
}
