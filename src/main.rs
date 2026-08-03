mod cpu;
mod gpu;
mod mode;
mod features;
mod device_spoof;
mod logger;
mod vm;
mod threads;
mod doze;
mod io_sched;
mod cgroups;
mod fas;
mod scx;
mod thermal;
mod analytics;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::process::Command;

use cpu::CpuManager;
use gpu::GpuManager;
use mode::ModeManager;
use vm::VmManager;
use threads::ThreadOptimizer;
use doze::DozeManager;
use io_sched::IoManager;
use cgroups::CgroupManager;
use fas::FasScheduler;
use thermal::{ThermalManager, ThermalSnapshot};
use analytics::AnalyticsCollector;
const MODULE_DIR: &str = "/data/adb/modules/yamitsuki_oplus";
const MODE_FILE: &str = "/data/local/tmp/yamitsuki_mode";
const CMD_FILE: &str = "/data/local/tmp/yamitsuki_cmd";
const GOVERNOR_INFO_FILE: &str = "/data/local/tmp/governor_info";
const GOVERNOR_SELECTED_FILE: &str = "/data/adb/modules/yamitsuki_oplus/governor_selected";
const SPOOF_RESULT_FILE: &str = "/data/adb/modules/yamitsuki_oplus/device_spoof_result.txt";
const GAME_LIST: &str = "/data/adb/modules/yamitsuki_oplus/game_list.txt";
const BATTERY_CAPACITY: &str = "/sys/class/power_supply/battery/capacity";
const BATTERY_VOLTAGE: &str = "/sys/class/power_supply/battery/voltage_now";
const BATTERY_CURRENT: &str = "/sys/class/power_supply/battery/current_now";
const BATTERY_POWER_NOW: &str = "/sys/class/power_supply/battery/power_now";
const BATTERY_POWER_AVG: &str = "/sys/class/power_supply/battery/power_avg";

fn read_sysfs(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default().trim().to_string()
}

struct PowerSampler {
    watts: [f64; 5],
    index: usize,
    count: usize,
}

impl PowerSampler {
    fn new() -> Self {
        Self {
            watts: [0.0; 5],
            index: 0,
            count: 0,
        }
    }

    /// 返回原始电压、电流、瞬时计算值、5 次移动均值与是否在充电（current_now 为负）。
    fn sample(&mut self) -> (String, String, String, String, bool) {
        let voltage_raw = read_sysfs(BATTERY_VOLTAGE);
        let current_raw = read_sysfs(BATTERY_CURRENT);
        let charging = current_raw.parse::<i64>().ok().unwrap_or(0) < 0;
        let voltage = voltage_raw.parse::<i64>().ok().unwrap_or(0).unsigned_abs() as f64;
        let current = current_raw.parse::<i64>().ok().unwrap_or(0).unsigned_abs() as f64;
        let instantaneous = if voltage > 100_000.0 && current > 100_000.0 {
            voltage * current / 1_000_000_000_000.0
        } else if voltage > 0.0 && current > 0.0 {
            voltage * current / 1_000_000.0
        } else { 0.0 };
        self.watts[self.index] = instantaneous;
        self.index = (self.index + 1) % self.watts.len();
        self.count = (self.count + 1).min(self.watts.len());
        let average = self.watts[..self.count].iter().sum::<f64>() / self.count as f64;
        (
            voltage_raw,
            current_raw,
            format!("{:.2}", instantaneous),
            format!("{:.2}", average),
            charging,
        )
    }
}

/// ── 命令管道事件循环 ──
fn cmd_loop(
    running: Arc<AtomicBool>,
    cpu: Arc<CpuManager>,
    _gpu: Arc<GpuManager>,
    mode_mgr: Arc<ModeManager>,
) {
    while running.load(Ordering::Relaxed) {
        let raw = match fs::read_to_string(CMD_FILE) {
            Ok(s) => s,
            Err(_) => {
                thread::sleep(Duration::from_millis(200));
                continue;
            }
        };
        let cmd = raw.trim().to_string();
        if cmd.is_empty() {
            thread::sleep(Duration::from_millis(200));
            continue;
        }

        let _ = fs::write(CMD_FILE, "");

        // ── governor commands ──
        if cmd == "governor:info" {
            let available = cpu.available_governors();
            let current = cpu.current_governor();
            let mut info = format!("current:{}\navailable:{}\n", current, available.join(","));
            if let Ok(saved) = fs::read_to_string(GOVERNOR_SELECTED_FILE) {
                if !saved.trim().is_empty() {
                    info.push_str(&format!("saved:{}\n", saved.trim()));
                }
            }
            let _ = fs::write(GOVERNOR_INFO_FILE, &info);
        } else if cmd == "governor:auto" {
            let _ = fs::write(GOVERNOR_SELECTED_FILE, "auto");
            let active = mode_mgr.active_mode();
            let gov = cpu.governor_for_mode(&active);
            cpu.set_all_governors(&gov);
        } else if cmd.starts_with("governor:set:") {
            let gov = &cmd["governor:set:".len()..];
            let _ = fs::write(GOVERNOR_SELECTED_FILE, gov);
            cpu.set_all_governors(gov);
        }
        // ── feature toggles ──
        else if cmd.starts_with("charge_boost:") {
            let enabled = cmd.ends_with("enable");
            let _ = fs::write(
                &format!("{}/charge_boost_enabled", MODULE_DIR),
                if enabled { "1" } else { "" },
            );
            features::charge_boost(enabled);
        } else if cmd.starts_with("horae:") {
            let enabled = cmd.ends_with("enable");
            let _ = fs::write(
                &format!("{}/horae_enabled", MODULE_DIR),
                if enabled { "1" } else { "" },
            );
            features::disable_horae(enabled);
        } else if cmd.starts_with("hw_overlay:") {
            let enabled = cmd.ends_with("enable");
            let _ = fs::write(
                &format!("{}/hw_overlay_enabled", MODULE_DIR),
                if enabled { "1" } else { "" },
            );
            features::hw_overlay(enabled);
        } else if cmd.starts_with("step_charging:") {
            let enabled = cmd.ends_with("enable");
            let _ = fs::write(
                &format!("{}/step_charging_enabled", MODULE_DIR),
                if enabled { "1" } else { "" },
            );
            features::step_charging(enabled);
        } else if cmd.starts_with("prop:") {
            let enabled = cmd.ends_with("enable");
            let _ = fs::write(
                &format!("{}/prop_enabled", MODULE_DIR),
                if enabled { "1" } else { "" },
            );
            features::prop_spoof(enabled);
        } else if cmd.starts_with("disable_usb:") {
            let enabled = cmd.ends_with("enable");
            let _ = fs::write(
                &format!("{}/disable_usb_enabled", MODULE_DIR),
                if enabled { "1" } else { "" },
            );
            features::disable_usb_debug(enabled);
        } else if cmd == "device_spoof:run" {
            let result = device_spoof::run_spoof();
            let _ = fs::write(SPOOF_RESULT_FILE, &result);
        }

        thread::sleep(Duration::from_millis(50));
    }
}

/// ── 主守护循环 ──
fn daemon_loop(
    cpu: Arc<CpuManager>,
    gpu: Arc<GpuManager>,
    mode_mgr: Arc<ModeManager>,
    vm: Arc<VmManager>,
    io: Arc<IoManager>,
    thermal: Arc<ThermalManager>,
    doze_mgr: Arc<Mutex<DozeManager>>,
    fas_sched: Arc<Mutex<FasScheduler>>,
    thread_opt: Arc<ThreadOptimizer>,
    cgroup: Arc<CgroupManager>,
) {
    let mut prev_mode = String::new();
    let mut pending_mode = String::new();
    let mut pending_mode_samples = 0u8;
    let mut was_dozing = false;
    let mut last_scx_status = String::new();
    let mut power_sampler = PowerSampler::new();
    let mut analytics = AnalyticsCollector::new();
let mut diag_tick: u32 = 0;
    let mut last_foreground = String::new();
    let mut last_top_processes = String::new();
    let mut gpu_protection_level: u8 = 0;
    let mut gpu_high_ticks: u8 = 0;
    let mut gpu_clear_ticks: u8 = 0;
    let mut thread_opt_tick: u8 = 0;
    let mut thread_degraded = false;
    let mut thread_hot_ticks: u8 = 0;
    let mut thread_cool_ticks: u8 = 0;
    let mut temp_history: [i64; 4] = [0; 4];
    let mut temp_hist_idx: usize = 0;
    let mut temp_hist_count: usize = 0;

    loop {
        let sampled_mode = mode_mgr.active_mode();
        if sampled_mode == pending_mode {
            pending_mode_samples = pending_mode_samples.saturating_add(1);
        } else {
            pending_mode = sampled_mode;
            pending_mode_samples = 1;
        }
        let initializing = prev_mode.is_empty();
        let mode_changed = pending_mode_samples >= 2 && pending_mode != prev_mode;
        let active = if mode_changed || initializing { pending_mode.clone() } else { prev_mode.clone() };
        if mode_changed {
            let reason = mode_mgr.decision_reason();
            logger::log(&format!("模式切换: {} -> {}，原因: {}", prev_mode, active, reason));
        }
        if mode_changed || initializing {
            if !mode_changed {
                logger::log(&format!("初始模式: {}，原因: {}", active, mode_mgr.decision_reason()));
            }
            prev_mode = active.clone();
            cpu.apply_mode(&active);
            if cpu.diagnostic_write_failures() > 0 {
                logger::log(&format!("CPU策略写入失败累计={}", cpu.diagnostic_write_failures()));
            }
            gpu.apply_mode(&active);
        }

        // ── PLG110 智能调度：CPU/GPU 负载 + SoC/CPU 温度 ──
        let cpu_load = cpu.load_percent();
        let (gpu_avg, gpu_current) = {
            let mut fas = fas_sched.lock().unwrap();
            (fas.update(), fas.current_gpu_freq())
        };
        let gpu_util_known = gpu_avg.is_some();
        let gpu_load = gpu_avg.unwrap_or(0);
        let thermal_snapshot = thermal.snapshot();
        let (voltage_raw, current_raw, power_inst, power_avg, charging) = power_sampler.sample();
        let power_watts = power_inst.parse::<f64>().unwrap_or(0.0);
        let power_avg_watts = power_avg.parse::<f64>().unwrap_or(0.0);
        analytics.record(&read_sysfs(BATTERY_CAPACITY), &voltage_raw, &current_raw, power_avg_watts, charging);
// 充电时 current_now 为负，V×I 表示充电功率而非系统功耗，不用于降级判定。
        let power_draw = if charging { 0.0 } else { power_avg_watts };
        if let Some(soc) = thermal_snapshot.soc_max {
            temp_history[temp_hist_idx] = soc;
            temp_hist_idx = (temp_hist_idx + 1) % temp_history.len();
            temp_hist_count = (temp_hist_count + 1).min(temp_history.len());
        }
        let smooth_temp = if temp_hist_count > 0 {
            temp_history[..temp_hist_count].iter().sum::<i64>() / temp_hist_count as i64
        } else { 0 };
        let gpu_temp_c = thermal_snapshot.gpu_max.map(|value| value as f64 / 1000.0).unwrap_or(0.0);
        let screen_off = mode_mgr.is_screen_off();
        let gpu_idle = !screen_off && cpu_load <= 25 && power_draw < 1.5;
        let gpu_high = gpu_util_known && !gpu_idle && gpu_load >= 85 && (gpu_temp_c >= 52.0 || power_draw >= 3.5);
        let gpu_clear = gpu_idle || (gpu_util_known && gpu_load <= 45 && gpu_temp_c > 0.0 && gpu_temp_c <= 48.0 && power_draw < 2.5);
        if gpu_high {
            gpu_high_ticks = gpu_high_ticks.saturating_add(1);
            gpu_clear_ticks = 0;
        } else if gpu_clear {
            gpu_clear_ticks = gpu_clear_ticks.saturating_add(1);
            gpu_high_ticks = 0;
        } else {
            gpu_high_ticks = 0;
            gpu_clear_ticks = 0;
        }
        let battery_level = read_sysfs(BATTERY_CAPACITY).parse::<u32>().unwrap_or(100);
        let requested_level = if screen_off || battery_level < 15 { 3 }
            else if gpu_high_ticks >= 3 { 2 }
            else if gpu_high_ticks >= 1 { 1 }
            else if gpu_clear_ticks >= 8 { 0 }
            else { gpu_protection_level };
        if requested_level != gpu_protection_level && (mode_changed || requested_level > 0 || gpu_clear_ticks >= 8) {
            if gpu.apply_protection(&active, requested_level) {
                gpu_protection_level = requested_level;
                logger::log(&format!("GPU保护级别切换: level={} load={} temp={:.1}C power={:.2}W max_freq={}MHz", requested_level, gpu_load, gpu_temp_c, power_watts, gpu.max_freq() / 1_000_000));
            }
        }
        let cap_permille = cpu.apply_dynamic_cap(&active, cpu_load, smooth_temp, power_draw);

        let valid_protection_temp = smooth_temp > 0;
        let high_thread_condition = valid_protection_temp && (smooth_temp >= 52_000 || power_draw >= 4.5);
        let cool_thread_condition = valid_protection_temp && smooth_temp <= 48_000 && power_draw < 3.5;
        if high_thread_condition {
            thread_hot_ticks = thread_hot_ticks.saturating_add(1);
            thread_cool_ticks = 0;
        } else if cool_thread_condition {
            thread_cool_ticks = thread_cool_ticks.saturating_add(1);
            thread_hot_ticks = 0;
        } else {
            thread_hot_ticks = 0;
            thread_cool_ticks = 0;
        }
        if !thread_degraded && thread_hot_ticks >= 3 {
            thread_degraded = true;
            logger::log(&format!("线程优化进入降级: temp={:.1}C power={:.2}W", smooth_temp as f64 / 1000.0, power_draw));
        } else if thread_degraded && thread_cool_ticks >= 6 {
            thread_degraded = false;
            logger::log(&format!("线程优化恢复: temp={:.1}C power={:.2}W", smooth_temp as f64 / 1000.0, power_draw));
        }

        thread_opt_tick = thread_opt_tick.wrapping_add(1);
        if active == "performance" && (mode_changed || thread_opt_tick % 10 == 0) {
            let game_list = fs::read_to_string(GAME_LIST).unwrap_or_default();
            for pkg in game_list.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                let pkg = if let Some(idx) = pkg.find('#') { &pkg[..idx] } else { pkg };
                thread_opt.optimize_game_with_policy(pkg, !thread_degraded, !thread_degraded);
                let moved = cgroup.assign_game(pkg);
                if moved == 0 {
                    logger::log(&format!("cgroup迁移失败: pkg={}", pkg));
                }
            }
        } else {
            let game_list = fs::read_to_string(GAME_LIST).unwrap_or_default();
            for pkg in game_list.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                let pkg = if let Some(idx) = pkg.find('#') { &pkg[..idx] } else { pkg };
                thread_opt.restore_game(pkg);
            }
        }

        // ── IO scheduler ──
        if mode_changed {
            let sched = IoManager::scheduler_for_mode(&active);
            io.apply(sched, &active);
        }

        // ── VM memory ──
        if mode_changed {
            vm.apply_mode(&active);
        }

        // ── Doze ──
        {
            let mut d = doze_mgr.lock().unwrap();
            if screen_off && !was_dozing {
                d.enter_doze();
                was_dozing = true;
                logger::log("进入深度休眠");
            } else if !screen_off && was_dozing {
                d.exit_doze();
                was_dozing = false;
                logger::log("退出深度休眠");
            }
        }

        // ── Thermal spoof ──
        if mode_changed {
            thermal.apply_spoof(&active);
        }

        // ── Logging ──
        let scx_status = scx::ScxManager::status_string();
        if scx_status != last_scx_status {
            logger::log(&format!("scx 状态: {}", scx_status));
            last_scx_status = scx_status.clone();
        }
        let soc_temp = thermal_snapshot.soc_max;
        let cpu_temp = thermal_snapshot.cpu_core_max;
        let gpu_temp = thermal_snapshot.gpu_max;
        let shell_front = thermal_snapshot.shell_front;
        let shell_frame = thermal_snapshot.shell_frame;
        let shell_back = thermal_snapshot.shell_back;
        let soc_temp_text = ThermalSnapshot::temp_celsius(soc_temp);
        let cpu_temp_text = ThermalSnapshot::temp_celsius(cpu_temp);
        let gpu_temp_text = ThermalSnapshot::temp_celsius(gpu_temp);
        let shell_front_text = ThermalSnapshot::temp_celsius(shell_front);
        let shell_frame_text = ThermalSnapshot::temp_celsius(shell_frame);
        let shell_back_text = ThermalSnapshot::temp_celsius(shell_back);
        let protection_temp_text = ThermalSnapshot::temp_celsius(thermal_snapshot.soc_max);
        diag_tick = diag_tick.wrapping_add(1);
        if diag_tick % 20 == 0 && !screen_off {
            last_top_processes = Command::new("sh").args(["-c", "top -b -n 1 -m 5 2>/dev/null | tail -5 | tr '\\n' ';'"]).output()
                .ok().map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string()).unwrap_or_default();
            last_foreground = Command::new("dumpsys").args(["activity", "activities"]).output()
                .ok().map(|out| String::from_utf8_lossy(&out.stdout).lines()
                    .find(|line| line.contains("mResumedActivity") || line.contains("topResumedActivity"))
                    .unwrap_or("").trim().to_string()).unwrap_or_default();
        }
        let zone_summary = &thermal_snapshot.zone_summary;
        let last_core = cpu.last_core().unwrap_or(0);
        logger::log_debug(&format!(
            "mode={} cpu0={}MHz cpu{}={}MHz cpu_load={}% cap={}‰ gpu={}MHz gpu_load={} gpu_control=devfreq_max_freq gpu_protect_level={} gpu_max={}MHz soc_max={}C cpu_core_max={}C gpu_max={}C shell_front={}C shell_frame={}C shell_back={}C thermal_protect={}C bat={}% charging={} v_raw={} i_raw={} p_inst={}W p_avg5={}W p_now_raw={} p_avg_raw={} io={} vm_sw={} foreground={} zones={} top={}",
            active,
            cpu.read_freq(0) / 1000,
            last_core,
            cpu.read_freq(last_core) / 1000,
            cpu_load,
            cap_permille,
            if gpu_current > 0 { gpu_current / 1_000_000 } else { gpu.current_freq() / 1_000_000 },
            gpu_load,
            gpu_protection_level,
            gpu.max_freq() / 1_000_000,
            soc_temp_text,
            cpu_temp_text,
            gpu_temp_text,
            shell_front_text,
            shell_frame_text,
            shell_back_text,
            protection_temp_text,
            read_sysfs(BATTERY_CAPACITY),
            charging,
            voltage_raw,
            current_raw,
            power_inst,
            power_avg,
            read_sysfs(BATTERY_POWER_NOW),
            read_sysfs(BATTERY_POWER_AVG),
            io.current(),
            vm.current_swappiness(),
            last_foreground,
            zone_summary,
            last_top_processes,
        ));

        thread::sleep(Duration::from_millis(1500));
    }
}

fn main() {
    logger::init();
    logger::log("══════ MoonTune V2.0 Rust 底层引擎启动 ══════");

    // ── 硬件发现 ──
    let cpu = Arc::new(CpuManager::new());
    let gpu = Arc::new(GpuManager::new());
    let mode_mgr = Arc::new(ModeManager::new());
    let vm = Arc::new(VmManager);
    let io = Arc::new(IoManager);
    let thermal = Arc::new(ThermalManager::new());
    let doze_mgr = Arc::new(Mutex::new(DozeManager::new()));
    let fas_sched = Arc::new(Mutex::new(FasScheduler::new()));
    let thread_opt = Arc::new(ThreadOptimizer::new(&cpu.big_cores));
    let cgroup = Arc::new(CgroupManager::new(&cpu.big_cores, &cpu.little_cores));

    logger::log(&format!(
        "CPU: {} cores ({} little + {} middle + {} big + {} prime) | GPU: {} | {:?}",
        cpu.little_cores.len() + cpu.middle_cores.len() + cpu.big_cores.len(),
        cpu.little_cores.len(),
        cpu.middle_cores.len(),
        cpu.big_cores.len(),
        cpu.prime_cores.len(),
        gpu.vendor_name(),
        cpu.available_governors(),
    ));

    // ── scx 检测 ──
    if scx::ScxManager::is_available() {
        let avail = scx::ScxManager::detect_available();
        logger::log(&format!(
            "风驰调度 (scx) 可用，检测到: {:?}",
            avail
        ));
        for s in &avail {
            if s == "scx_bpfland" {
                if scx::ScxManager::load_scheduler("scx_bpfland") {
                    logger::log("风驰调度已加载: scx_bpfland");
                }
            }
        }
    } else {
        logger::log("风驰调度 (scx) 不可用，使用传统 governor");
    }

    // ── cgroups 初始化 ──
    cgroup.init();

    // ── 恢复持久化 feature 状态 ──
    let feature_flags = [
        "charge_boost_enabled",
        "horae_enabled",
        "hw_overlay_enabled",
        "step_charging_enabled",
        "prop_enabled",
        "disable_usb_enabled",
    ];
    for f in &feature_flags {
        let path = format!("{}/{}", MODULE_DIR, f);
        if Path::new(&path).exists() {
            match *f {
                "charge_boost_enabled" => features::charge_boost(true),
                "horae_enabled" => features::disable_horae(true),
                "hw_overlay_enabled" => features::hw_overlay(true),
                "step_charging_enabled" => features::step_charging(true),
                "prop_enabled" => features::prop_spoof(true),
                "disable_usb_enabled" => features::disable_usb_debug(true),
                _ => {}
            }
        }
    }

    // ── 恢复 governor 偏好 ──
    if let Ok(saved) = fs::read_to_string(GOVERNOR_SELECTED_FILE) {
        let saved = saved.trim().to_string();
        if !saved.is_empty() && saved != "auto" {
            cpu.set_all_governors(&saved);
        }
    }

    // ── 初始 IO 调度 ──
    let init_mode = mode_mgr.active_mode();
    let init_sched = IoManager::scheduler_for_mode(&init_mode);
    io.apply(init_sched, &init_mode);
    vm.apply_mode(&init_mode);

    logger::log(&format!("初始模式: {} | IO: {} | VM swapp={}", init_mode, io.current(), vm.current_swappiness()));

    // ── 命令管道线程 ──
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        let c = cpu.clone();
        let g = gpu.clone();
        let m = mode_mgr.clone();
        thread::spawn(move || cmd_loop(r, c, g, m));
    }

    // ── 主循环（永不返回） ──
    daemon_loop(cpu, gpu, mode_mgr, vm, io, thermal, doze_mgr, fas_sched, thread_opt, cgroup);
}
