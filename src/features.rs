use std::fs;
use std::process::Command;

/// Unlock maximum charging current
pub fn charge_boost(enabled: bool) {
    let path = "/sys/class/power_supply/battery/constant_charge_current";
    if enabled {
        let _ = fs::write(path, "6000000");
    } else {
        let _ = fs::write(path, "3000000");
    }
}

/// Disable Horae thermal engine for better charging speed
pub fn disable_horae(enabled: bool) {
    let val = if enabled { "0" } else { "1" };
    let _ = Command::new("setprop")
        .args(&["persist.sys.horae.enable", val])
        .output();
}

/// Disable hardware overlay for smoother rendering
pub fn hw_overlay(enabled: bool) {
    let val = if enabled { "1" } else { "0" };
    let _ = Command::new("settings")
        .args(&[
            "put",
            "global",
            "hwui.disable_vsync",
            val,
        ])
        .output();
}

/// Disable step charging for consistent high-speed charge
pub fn step_charging(enabled: bool) {
    let path = "/sys/class/power_supply/battery/step_charging_enabled";
    let val = if enabled { "0" } else { "1" };
    let _ = fs::write(path, val);
}

/// Reset system properties for spoofing
pub fn prop_spoof(enabled: bool) {
    let props = &[
        "ro.boot.vbmeta.device_state",
        "ro.boot.verifiedbootstate",
        "ro.boot.flash.locked",
        "ro.boot.veritymode",
        "ro.boot.warranty_bit",
        "ro.warranty_bit",
        "ro.debuggable",
        "ro.force.debuggable",
        "ro.secure",
        "ro.adb.secure",
        "ro.build.type",
        "ro.build.tags",
        "ro.vendor.boot.warranty_bit",
        "ro.vendor.warranty_bit",
        "vendor.boot.vbmeta.device_state",
        "vendor.boot.verifiedbootstate",
        "sys.oem_unlock_allowed",
        "ro.secureboot.lockstate",
    ];

    for prop in props {
        if enabled {
            // Get current value and spoof
            if let Ok(output) = Command::new("getprop").arg(prop).output() {
                let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !val.is_empty() && val != "unknown" {
                    let _ = Command::new("resetprop")
                        .arg("-n")
                        .arg(prop)
                        .arg(match prop {
                            &"ro.boot.verifiedbootstate" | &"vendor.boot.verifiedbootstate" => "green",
                            &"ro.boot.flash.locked" => "1",
                            &"ro.boot.veritymode" => "enforcing",
                            &"ro.debuggable" | &"ro.force.debuggable" => "0",
                            &"ro.secure" | &"ro.adb.secure" => "1",
                            &"ro.build.type" => "user",
                            &"ro.build.tags" => "release-keys",
                            &"sys.oem_unlock_allowed" => "0",
                            _ => "locked",
                        })
                        .output();
                }
            }
        } else {
            // Restore original value
            if let Ok(output) = Command::new("getprop").arg(prop).output() {
                let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !val.is_empty() && val != "unknown" {
                    let _ = Command::new("resetprop")
                        .arg(prop)
                        .arg(&val)
                        .output();
                }
            }
        }
    }
}

/// Disable USB debugging
pub fn disable_usb_debug(enabled: bool) {
    let val = if enabled { "0" } else { "1" };
    let _ = Command::new("settings")
        .args(&["put", "global", "adb_enabled", val])
        .output();
    let _ = Command::new("settings")
        .args(&["put", "global", "development_settings_enabled", val])
        .output();
}
