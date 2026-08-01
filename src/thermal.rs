use std::fs;

const THERMAL_BASE: &str = "/sys/class/thermal";
const THERMAL_ZONE0: &str = "/sys/class/thermal/thermal_zone0/temp";
const THERMAL_ZONE1: &str = "/sys/class/thermal/thermal_zone1/temp";
const THERMAL_ZONE2: &str = "/sys/class/thermal/thermal_zone2/temp";

pub struct ThermalManager {
    original: i64,
}

impl ThermalManager {
    pub fn new() -> Self {
        let original = fs::read_to_string(THERMAL_ZONE0)
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(35_000);
        ThermalManager { original }
    }

    /// Read current real temperature (milli-degrees C)
    pub fn real_temp(&self) -> i64 {
        fs::read_to_string(THERMAL_ZONE0)
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(self.original)
    }

    /// Get spoofed temperature based on mode
    /// powersave → 28°C | balance → 35°C | performance → 42°C
    pub fn spoof_temp(&self, mode: &str) -> i64 {
        match mode {
            "powersave" => 28_000,
            "balance" => 35_000,
            "performance" => 42_000,
            _ => 35_000,
        }
    }

    /// Apply temperature spoofing by writing to thermal zone files
    /// Note: On some devices, thermal zones are read-only.
    /// On OPLUS devices with KernelSU, we can write to certain nodes.
    pub fn apply_spoof(&self, mode: &str) -> bool {
        let target = self.spoof_temp(mode).to_string();
        let mut success = false;

        // Try all common thermal zone temp files
        for zone in &[THERMAL_ZONE0, THERMAL_ZONE1, THERMAL_ZONE2] {
            if fs::write(zone, &target).is_ok() {
                success = true;
            }
            // Also try virtual thermal node (some custom kernels)
            let zone_virt = format!("{}/virtual_temp", zone.trim_end_matches("/temp"));
            let _ = fs::write(&zone_virt, &target);
        }

        // Try specific OPLUS thermal paths from binary
        let oplus_paths = &[
            "/sys/class/thermal/thermal_message/sconfig",
            "/sys/kernel/oplus_thermal/fake_temp",
        ];
        for p in oplus_paths {
            if fs::write(p, &target).is_ok() {
                success = true;
            }
        }

        success
    }

    /// Restore temperature to real reading
    pub fn restore(&self) {
        let real = self.real_temp().to_string();
        for zone in &[THERMAL_ZONE0, THERMAL_ZONE1, THERMAL_ZONE2] {
            let _ = fs::write(zone, &real);
        }
    }

    pub fn cpu_temp(&self) -> f64 {
        self.real_temp() as f64 / 1000.0
    }
}
