use std::fs;
use std::path::Path;

const SCX_ROOT: &str = "/sys/kernel/sched_ext";
const SCX_OPS: &str = "/sys/kernel/sched_ext/root/ops";

/// 风驰调度检测（scx — sched_ext）
/// 检测内核是否支持 sched_ext，并自动加载可用调度器
pub struct ScxManager;

impl ScxManager {
    /// Check if sched_ext is available in kernel
    pub fn is_available() -> bool {
        Path::new(SCX_ROOT).exists()
    }

    /// Get currently loaded scx scheduler
    pub fn current_scx() -> Option<String> {
        fs::read_to_string(SCX_OPS)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Check if scx is currently active
    pub fn is_active() -> bool {
        Self::current_scx().is_some()
    }

    /// Try to load a specific scx scheduler
    /// Common schedulers: scx_bpfland, scx_lavd, scx_rusty
    pub fn load_scheduler(name: &str) -> bool {
        // scx schedulers are loaded via bpftool or direct attach
        // Try scanning for available schedulers in common paths
        let paths = &[
            format!("/data/adb/modules/yamitsuki_oplus/{}", name),
            format!("/system/bin/{}", name),
            format!("/vendor/bin/{}", name),
            format!("/data/local/tmp/{}", name),
        ];

        for path in paths {
            if Path::new(path).exists() {
                // Try to start the scheduler
                let output = std::process::Command::new(&path)
                    .output()
                    .ok();
                return output
                    .map(|o| o.status.success())
                    .unwrap_or(false);
            }
        }
        false
    }

    /// Auto-detect available scx schedulers on the device
    pub fn detect_available() -> Vec<String> {
        if !Self::is_available() {
            return vec![];
        }

        let candidates = &[
            "scx_bpfland",
            "scx_lavd",
            "scx_rusty",
            "scx_simple",
            "scx_flatcg",
            "scx_nest",
        ];

        candidates
            .iter()
            .filter(|name| {
                Path::new(&format!("/system/bin/{}", name)).exists()
                    || Path::new(&format!("/vendor/bin/{}", name)).exists()
                    || Path::new(&format!("/data/local/tmp/{}", name)).exists()
                    || Path::new(&format!(
                        "/data/adb/modules/yamitsuki_oplus/{}",
                        name
                    ))
                    .exists()
            })
            .map(|s| s.to_string())
            .collect()
    }

    pub fn status_string() -> String {
        if !Self::is_available() {
            return "内核不支持 scx".to_string();
        }
        if let Some(ops) = Self::current_scx() {
            format!("已加载: {}", ops)
        } else {
            let available = Self::detect_available();
            if available.is_empty() {
                "scx 可用，未找到调度器".to_string()
            } else {
                format!("待加载: {}", available.join(", "))
            }
        }
    }
}
