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
use thermal::ThermalManager;

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

    /// 返回原始电压、电流、瞬时计算值和 5 次移动均值。PLG110 使用 mV/mA。
    fn sample(&mut self) -> (String, String, String, String) {
        let voltage_raw = read_sysfs(BATTERY_VOLTAGE);
        let current_raw = read_sysfs(BATTERY_CURRENT);
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
        } else if cmd.starts_with("mt_hide:") {
            let enabled = cmd.ends_with("enable");
            let _ = fs::write(
                &format!("{}/mt_hide_enabled", MODULE_DIR),
                if enabled { "1" } else { "" },
            );
            features::mt_hide(enabled);
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
    let mut was_dozing = false;
    let mut last_scx_status = String::new();
    let mut power_sampler = PowerSampler::new();
    let mut diag_tick: u32 = 0;
    let mut last_foreground = String::new();
    let mut last_top_processes = String::new();

    loop {
        let active = mode_mgr.active_mode();

        // ── Mode transition handling ──
        let mode_changed = active != prev_mode;
        if mode_changed {
            logger::log(&format!("模式切换: {} -> {}", prev_mode, active));
            prev_mode = active.clone();
        }

        // ── Core/GPU 基础模式控制 ──
        // 不执行 drop_caches：PLG110 的 UFS 重新读盘会抵消省电收益。
        cpu.apply_mode(&active);
        gpu.apply_mode(&active);

        // ── PLG110 智能调度：CPU/GPU 负载 + SoC/CPU 温度 ──
        let cpu_load = cpu.load_percent();
        let (gpu_avg, gpu_current) = {
            let mut fas = fas_sched.lock().unwrap();
            (fas.update(), fas.current_gpu_freq())
        };
        let protection_temp = thermal.max_protection_temp();
        let cap_permille = cpu.apply_dynamic_cap(&active, cpu_load, protection_temp);

        // ── Thread optimization (game mode only) ──
        if active == "performance" {
            let game_list = fs::read_to_string(GAME_LIST).unwrap_or_default();
            for pkg in game_list.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                let pkg = if let Some(idx) = pkg.find('#') { &pkg[..idx] } else { pkg };
                thread_opt.optimize_game(pkg);
                cgroup.assign_game(pkg);
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
        let screen_off = mode_mgr.is_screen_off();
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
        let (voltage_raw, current_raw, power_inst, power_avg) = power_sampler.sample();
        let soc_temp = thermal.real_temp() as f64 / 1000.0;
        let cpu_temp = thermal.cpu_core_temp().unwrap_or(-1.0);
        let gpu_temp = thermal.gpu_temp().unwrap_or(-1.0);
        diag_tick = diag_tick.wrapping_add(1);
        if diag_tick % 4 == 0 {
            last_top_processes = Command::new("sh").args(["-c", "top -b -n 1 -m 5 2>/dev/null | tail -5 | tr '\\n' ';'"]).output()
                .ok().map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string()).unwrap_or_default();
            last_foreground = Command::new("dumpsys").args(["activity", "activities"]).output()
                .ok().map(|out| String::from_utf8_lossy(&out.stdout).lines()
                    .find(|line| line.contains("mResumedActivity") || line.contains("topResumedActivity"))
                    .unwrap_or("").trim().to_string()).unwrap_or_default();
        }
        let zone_summary = thermal.zone_summary();
        logger::log(&format!(
            "mode={} cpu0={}MHz cpu7={}MHz cpu_load={}% cap={}‰ gpu={}MHz gpu_load={} gpu_control=observe soc={:.1}C cpu_temp={:.1}C gpu_temp={:.1}C thermal_protect={:.1}C bat={}% v_raw={} i_raw={} p_inst={}W p_avg5={}W p_now_raw={} p_avg_raw={} io={} vm_sw={} foreground={} zones={} top={}",
            active,
            cpu.read_freq(0) / 1000,
            cpu.read_freq(7) / 1000,
            cpu_load,
            cap_permille,
            if gpu_current > 0 { gpu_current / 1_000_000 } else { gpu.current_freq() / 1_000_000 },
            gpu_avg,
            soc_temp,
            cpu_temp,
            gpu_temp,
            protection_temp as f64 / 1000.0,
            read_sysfs(BATTERY_CAPACITY),
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
    logger::log("══════ YamiTsuki V2.0 Rust 底层引擎启动 ══════");

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
        "CPU: {} cores ({} big + {} little) | GPU: {} | {:?}",
        cpu.big_cores.len() + cpu.little_cores.len(),
        cpu.big_cores.len(),
        cpu.little_cores.len(),
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
        "mt_hide_enabled",
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
                "mt_hide_enabled" => features::mt_hide(true),
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
