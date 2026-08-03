use std::fs;

pub const CONFIG_FILE: &str = "/data/adb/modules/yamitsuki_oplus/profiles.conf";

#[derive(Clone)]
pub struct ModeConfig {
    pub governor: String,
    pub gpu_governor: String,
    pub cpu_little: f64,
    pub cpu_middle: f64,
    pub cpu_prime: f64,
    pub cpu_dynamic_base: f64,
    pub cpu_dynamic_max: f64,
    pub gpu_ratio: f64,
    pub gpu_protect_1: u64,
    pub gpu_protect_2: u64,
    pub gpu_protect_3: u64,
    pub vm_swappiness: u32,
    pub vm_dirty_ratio: u32,
    pub vm_dirty_background_ratio: u32,
    pub vm_dirty_writeback: u32,
    pub vm_dirty_expire: u32,
    pub vm_vfs_cache: u32,
    pub vm_overcommit: u32,
    pub io_scheduler: String,
    pub io_read_ahead: u32,
    pub io_nr_requests: u32,
    pub io_rq_affinity: u32,
    pub io_nomerges: u32,
    pub thermal_spoof: i64,
    pub gpu_high_load: u32,
    pub gpu_high_temp: f64,
    pub gpu_high_power: f64,
    pub gpu_clear_load: u32,
    pub gpu_clear_temp: f64,
    pub gpu_clear_power: f64,
    pub thread_hot_temp: i64,
    pub thread_hot_power: f64,
    pub thread_cool_temp: i64,
    pub thread_cool_power: f64,
}

fn defaults(mode: &str) -> ModeConfig {
    let (cpu_little, cpu_middle, cpu_prime, base, max, gpu_ratio, swappiness, dirty_ratio, dirty_bg, writeback, expire, cache, overcommit, read_ahead, requests, thermal) = match mode {
        "powersave" => (.55, .60, .65, .55, .65, .35, 15, 10, 3, 1000, 2000, 80, 0, 128, 64, 28_000),
        "performance" => (1.0, 1.0, 1.0, .85, 1.0, 1.0, 10, 40, 10, 3000, 6000, 30, 1, 1024, 512, 42_000),
        _ => (.8, .75, .7, .65, .85, .75, 40, 10, 5, 500, 3000, 60, 1, 256, 128, 35_000),
    };
    ModeConfig { governor: "auto".into(), gpu_governor: "auto".into(), cpu_little, cpu_middle, cpu_prime, cpu_dynamic_base: base, cpu_dynamic_max: max, gpu_ratio, gpu_protect_1: 780_000_000, gpu_protect_2: 650_000_000, gpu_protect_3: 520_000_000, vm_swappiness: swappiness, vm_dirty_ratio: dirty_ratio, vm_dirty_background_ratio: dirty_bg, vm_dirty_writeback: writeback, vm_dirty_expire: expire, vm_vfs_cache: cache, vm_overcommit: overcommit, io_scheduler: "mq-deadline".into(), io_read_ahead: read_ahead, io_nr_requests: requests, io_rq_affinity: 2, io_nomerges: 0, thermal_spoof: thermal, gpu_high_load: 85, gpu_high_temp: 52.0, gpu_high_power: 3.5, gpu_clear_load: 45, gpu_clear_temp: 48.0, gpu_clear_power: 2.5, thread_hot_temp: 52_000, thread_hot_power: 4.5, thread_cool_temp: 48_000, thread_cool_power: 3.5 }
}

fn value(raw: &str, key: &str) -> Option<String> {
    raw.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')).find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().to_string())
    })
}

fn parsed<T: std::str::FromStr>(raw: &str, key: &str, fallback: T) -> T {
    value(raw, key).and_then(|item| item.parse().ok()).unwrap_or(fallback)
}

macro_rules! load_number {
    ($raw:expr, $cfg:expr, $field:ident, $key:expr) => {
        $cfg.$field = parsed($raw, &$key, $cfg.$field);
    };
}

pub fn load(mode: &str) -> ModeConfig {
    let mode = mode.trim();
    let mut cfg = defaults(mode);
    let raw = fs::read_to_string(CONFIG_FILE).unwrap_or_default();
    let key = |name: &str| format!("{}.{}", mode, name);
    cfg.governor = value(&raw, &key("governor")).unwrap_or_else(|| cfg.governor.clone());
    cfg.gpu_governor = value(&raw, &key("gpu_governor")).unwrap_or_else(|| cfg.gpu_governor.clone());
    load_number!(&raw, cfg, cpu_little, key("cpu_little")); load_number!(&raw, cfg, cpu_middle, key("cpu_middle")); load_number!(&raw, cfg, cpu_prime, key("cpu_prime"));
    load_number!(&raw, cfg, cpu_dynamic_base, key("cpu_dynamic_base")); load_number!(&raw, cfg, cpu_dynamic_max, key("cpu_dynamic_max")); load_number!(&raw, cfg, gpu_ratio, key("gpu_ratio"));
    load_number!(&raw, cfg, gpu_protect_1, key("gpu_protect_1")); load_number!(&raw, cfg, gpu_protect_2, key("gpu_protect_2")); load_number!(&raw, cfg, gpu_protect_3, key("gpu_protect_3"));
    load_number!(&raw, cfg, vm_swappiness, key("vm_swappiness")); load_number!(&raw, cfg, vm_dirty_ratio, key("vm_dirty_ratio")); load_number!(&raw, cfg, vm_dirty_background_ratio, key("vm_dirty_background_ratio")); load_number!(&raw, cfg, vm_dirty_writeback, key("vm_dirty_writeback")); load_number!(&raw, cfg, vm_dirty_expire, key("vm_dirty_expire")); load_number!(&raw, cfg, vm_vfs_cache, key("vm_vfs_cache")); load_number!(&raw, cfg, vm_overcommit, key("vm_overcommit"));
    cfg.io_scheduler = value(&raw, &key("io_scheduler")).unwrap_or_else(|| cfg.io_scheduler.clone()); load_number!(&raw, cfg, io_read_ahead, key("io_read_ahead")); load_number!(&raw, cfg, io_nr_requests, key("io_nr_requests")); load_number!(&raw, cfg, io_rq_affinity, key("io_rq_affinity")); load_number!(&raw, cfg, io_nomerges, key("io_nomerges")); load_number!(&raw, cfg, thermal_spoof, key("thermal_spoof"));
    load_number!(&raw, cfg, gpu_high_load, key("gpu_high_load")); load_number!(&raw, cfg, gpu_high_temp, key("gpu_high_temp")); load_number!(&raw, cfg, gpu_high_power, key("gpu_high_power")); load_number!(&raw, cfg, gpu_clear_load, key("gpu_clear_load")); load_number!(&raw, cfg, gpu_clear_temp, key("gpu_clear_temp")); load_number!(&raw, cfg, gpu_clear_power, key("gpu_clear_power")); load_number!(&raw, cfg, thread_hot_temp, key("thread_hot_temp")); load_number!(&raw, cfg, thread_hot_power, key("thread_hot_power")); load_number!(&raw, cfg, thread_cool_temp, key("thread_cool_temp")); load_number!(&raw, cfg, thread_cool_power, key("thread_cool_power"));
    cfg
}
