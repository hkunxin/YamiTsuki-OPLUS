use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const DATA_DIR: &str = "/data/adb/modules/yamitsuki_oplus/data";
const STATUS_FILE: &str = "/data/local/tmp/yamitsuki_power_status";
const SAMPLE_INTERVAL_SECS: u64 = 30;
const APP_REFRESH_INTERVAL_SECS: u64 = 60;
const MAX_HISTORY_BYTES: u64 = 512 * 1024;

pub struct AnalyticsCollector {
    last_sample: u64,
    last_app_refresh: u64,
    foreground_package: String,
    package_labels: HashMap<String, String>,
}

impl AnalyticsCollector {
    pub fn new() -> Self {
        let _ = fs::create_dir_all(DATA_DIR);
        Self {
            last_sample: 0,
            last_app_refresh: 0,
            foreground_package: String::new(),
            package_labels: HashMap::new(),
        }
    }

    pub fn record(&mut self, capacity: &str, voltage: &str, current: &str, power_w: f64, charging: bool) {
        let now = epoch_seconds();
        if now.saturating_sub(self.last_sample) < SAMPLE_INTERVAL_SECS {
            return;
        }
        self.last_sample = now;
        if now.saturating_sub(self.last_app_refresh) >= APP_REFRESH_INTERVAL_SECS {
            self.foreground_package = foreground_package();
            self.last_app_refresh = now;
        }

        let status = format!(
            "timestamp={}\ncapacity={}\nvoltage_raw={}\ncurrent_raw={}\npower_w={:.3}\ncharging={}\nforeground={}\n",
            now,
            sanitize(capacity),
            sanitize(voltage),
            sanitize(current),
            power_w,
            charging,
            sanitize(&self.foreground_package),
        );
        atomic_write(STATUS_FILE, &status);

        let date = day_key(now);
        let power_path = format!("{}/power-{}.csv", DATA_DIR, date);
        append_limited(&power_path, "timestamp,capacity,voltage_raw,current_raw,power_w,charging,foreground\n", &format!(
            "{},{},{},{},{:.3},{},{}\n",
            now,
            csv_field(capacity),
            csv_field(voltage),
            csv_field(current),
            power_w,
            charging,
            csv_field(&self.foreground_package),
        ));

        if !charging && power_w > 0.0 && !self.foreground_package.is_empty() {
            let app_path = format!("{}/apps-{}.csv", DATA_DIR, date);
            let foreground = self.foreground_package.clone();
            let label = self.app_label(&foreground);
            let estimated_mah = estimate_mah(power_w, voltage, SAMPLE_INTERVAL_SECS);
            append_limited(&app_path, "timestamp,package,label,estimated_power_w,estimated_mah,window_seconds,source\n", &format!(
                "{},{},{},{:.3},{:.4},{},foreground_attribution\n",
                now,
                csv_field(&self.foreground_package),
                csv_field(&label),
                power_w,
                estimated_mah,
                SAMPLE_INTERVAL_SECS,
            ));
        }
    }

    fn app_label(&mut self, package: &str) -> String {
        if let Some(label) = self.package_labels.get(package) {
            return label.clone();
        }
        let command = format!("cmd package resolve-activity --brief {} 2>/dev/null | tail -n 1", shell_quote(package));
        let output = Command::new("sh").args(["-c", &command]).output().ok()
            .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| package.to_string());
        self.package_labels.insert(package.to_string(), output.clone());
        output
    }
}

fn epoch_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_secs()).unwrap_or(0)
}

fn day_key(timestamp: u64) -> String {
    let output = Command::new("date").args(["-d", &format!("@{}", timestamp), "+%Y%m%d"]).output().ok()
        .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string())
        .unwrap_or_default();
    if output.len() == 8 { output } else { "current".to_string() }
}

fn foreground_package() -> String {
    let command = "dumpsys window 2>/dev/null | grep -E 'mCurrentFocus|mFocusedApp' | head -1 | grep -oE '[a-zA-Z][a-zA-Z0-9._]*\\.[a-zA-Z][a-zA-Z0-9._]*' | head -1";
    Command::new("sh").args(["-c", command]).output().ok()
        .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string())
        .unwrap_or_default()
}

fn estimate_mah(power_w: f64, voltage_raw: &str, seconds: u64) -> f64 {
    let voltage = voltage_raw.trim().parse::<f64>().unwrap_or(0.0);
    let volts = if voltage > 100_000.0 { voltage / 1_000_000.0 } else if voltage > 100.0 { voltage / 1_000.0 } else { 0.0 };
    if volts > 0.0 { power_w / volts * 1000.0 * seconds as f64 / 3600.0 } else { 0.0 }
}

fn append_limited(path: &str, header: &str, row: &str) {
    if fs::metadata(path).map(|meta| meta.len() > MAX_HISTORY_BYTES).unwrap_or(false) {
        let _ = fs::rename(path, format!("{}.1", path));
    }
    let needs_header = fs::metadata(path).map(|meta| meta.len() == 0).unwrap_or(true);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        if needs_header { let _ = file.write_all(header.as_bytes()); }
        let _ = file.write_all(row.as_bytes());
    }
}

fn atomic_write(path: &str, content: &str) {
    let temporary = format!("{}.tmp", path);
    if fs::write(&temporary, content).is_ok() {
        let _ = fs::rename(temporary, path);
    }
}

fn sanitize(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

fn csv_field(value: &str) -> String {
    sanitize(value).replace(',', " ")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
