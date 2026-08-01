use std::fs;
use std::process::Command;

const MODE_FILE: &str = "/data/local/tmp/yamitsuki_mode";
const GAME_LIST: &str = "/data/adb/modules/yamitsuki_oplus/game_list.txt";
const BATTERY_CAPACITY: &str = "/sys/class/power_supply/battery/capacity";
const BATTERY_STATUS: &str = "/sys/class/power_supply/battery/status";
const SCREEN_POWER: &str = "/sys/class/backlight/panel0-backlight/bl_power";

pub struct ModeManager;

impl ModeManager {
    pub fn new() -> Self {
        ModeManager
    }

    pub fn active_mode(&self) -> String {
        let mode = fs::read_to_string(MODE_FILE)
            .unwrap_or_else(|_| "auto".to_string())
            .trim()
            .to_string();

        if mode == "auto" {
            // Smart mode logic
            let screen_off = self.is_screen_off();
            let battery_low = self.battery_level() < 15;
            let charging = self.is_charging();
            let game_running = self.is_game_running();

            if game_running {
                "performance".to_string()
            } else if screen_off || battery_low {
                "powersave".to_string()
            } else if charging {
                "balance".to_string()
            } else {
                "balance".to_string()
            }
        } else {
            mode
        }
    }

    fn battery_level(&self) -> u32 {
        fs::read_to_string(BATTERY_CAPACITY)
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(100)
    }

    fn is_charging(&self) -> bool {
        let status = fs::read_to_string(BATTERY_STATUS)
            .unwrap_or_default()
            .trim()
            .to_string();
        status == "Charging" || status == "Full"
    }

    pub fn is_screen_off(&self) -> bool {
        self.check_screen_off()
    }

    fn check_screen_off(&self) -> bool {
        if let Ok(val) = fs::read_to_string(SCREEN_POWER) {
            return val.trim() != "0";
        }
        if let Ok(output) = Command::new("dumpsys")
            .args(&["display"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return !stdout.contains("mScreenState=ON");
        }
        false
    }

    fn is_game_running(&self) -> bool {
        let list = match fs::read_to_string(GAME_LIST) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let pkgs: Vec<&str> = list
            .lines()
            .map(|l| {
                let l = l.trim();
                if let Some(idx) = l.find('#') {
                    &l[..idx]
                } else {
                    l
                }
            })
            .filter(|l| !l.is_empty())
            .collect();

        for pkg in &pkgs {
            if let Ok(output) = Command::new("pidof").arg(pkg).output() {
                if !output.stdout.is_empty() {
                    return true;
                }
            }
        }
        false
    }
}
