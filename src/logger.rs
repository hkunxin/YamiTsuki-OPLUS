use std::fs::{self, OpenOptions};
use std::io::Write;

const LOG_FILE: &str = "/data/adb/modules/yamitsuki_oplus/yamitsuki.log";
const CONFIG_FILE: &str = "/data/adb/modules/yamitsuki_oplus/log_config.conf";
const MAX_LOG_BYTES: u64 = 1024 * 1024;

pub fn init() {
    rotate_if_needed();
}

pub fn log(msg: &str) {
    write("INFO", msg);
}

pub fn log_debug(msg: &str) {
    write("DEBUG", msg);
}

fn write(level: &str, msg: &str) {
    rotate_if_needed();
    let configured_level = log_level();
    if configured_level == "off" || (level == "DEBUG" && configured_level != "debug") {
        return;
    }
    let ts = now_formatted();
    let line = format!("{} [{}] {}\n", ts, level, msg);

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(LOG_FILE) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn config_value(key: &str) -> Option<String> {
    fs::read_to_string(CONFIG_FILE).ok()?.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim() == key).then(|| value.trim().to_string())
        })
}

fn log_level() -> String {
    match config_value("LOG_LEVEL").as_deref() {
        Some("off") => "off".to_string(),
        Some("info") => "info".to_string(),
        _ => "debug".to_string(),
    }
}

fn timezone_offset_hours() -> i64 {
    config_value("TIMEZONE_OFFSET").and_then(|value| value.parse().ok()).unwrap_or(8)
}

fn rotate_if_needed() {
    if let Ok(meta) = fs::metadata(LOG_FILE) {
        if meta.len() >= MAX_LOG_BYTES {
            let old = format!("{}.old", LOG_FILE);
            let _ = fs::remove_file(&old);
            let _ = fs::rename(LOG_FILE, old);
        }
    }
}

fn now_formatted() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let offset = timezone_offset_hours().clamp(-23, 23) * 3600;
    let local_secs = secs.saturating_add(offset);
    let days = local_secs.div_euclid(86400);
    let day_secs = local_secs.rem_euclid(86400);
    let hour = day_secs / 3600;
    let min = (day_secs % 3600) / 60;
    let (y, mo, d) = epoch_to_date(days);
    let sign = if offset >= 0 { '+' } else { '-' };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:00 {}{:02}:00", y, mo, d, hour, min, sign, offset.abs() / 3600)
}

fn epoch_to_date(days: i64) -> (i64, u32, u32) {
    let mut d = days;
    let mut y = 1970i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 1u32;
    for &md in months.iter() {
        let md = if mo == 2 && is_leap(y) { 29 } else { md as i64 };
        if d < md {
            break;
        }
        d -= md;
        mo += 1;
    }
    (y, mo, d as u32 + 1)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
