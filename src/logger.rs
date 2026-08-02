use std::fs::{self, OpenOptions};
use std::io::Write;

const LOG_FILE: &str = "/data/adb/modules/yamitsuki_oplus/yamitsuki.log";
const MAX_LOG_BYTES: u64 = 256 * 1024;

pub fn init() {
    rotate_if_needed();
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

pub fn log(msg: &str) {
    rotate_if_needed();
    let ts = now_formatted();
    let line = format!("{} {}\n", ts, msg);

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(LOG_FILE) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn now_formatted() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let hm = (secs % 86400) / 60;
    let hour = hm / 60;
    let min = hm % 60;
    // Rough date from epoch: 1970-01-01 + days
    // Year/Month approximation from known epoch
    let (y, mo, d) = epoch_to_date(days as i64);
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, mo, d, hour, min)
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
