use std::fs;

const BLOCK_BASE: &str = "/sys/block";

pub enum IoScheduler { MqDeadline, Deadline, Bfq, Noop }
pub struct IoManager;

/// 只返回真实存储设备的 scheduler 路径（跳过 dm/loop/ram/zram 虚拟设备）
fn real_queue_files() -> Vec<String> {
    let Ok(entries) = fs::read_dir(BLOCK_BASE) else { return Vec::new(); };
    entries.filter_map(|entry| {
        let name = entry.ok()?.file_name().to_string_lossy().into_owned();
        // 只处理真实块设备: sd*(UFS/SCSI), mmcblk*(eMMC), nvme*
        if !name.starts_with("sd") && !name.starts_with("mmcblk") && !name.starts_with("nvme") {
            return None;
        }
        let path = format!("{}/{}/queue/scheduler", BLOCK_BASE, name);
        fs::metadata(&path).ok().map(|_| path)
    }).collect()
}

/// 检查某个调度器是否在可用列表中
fn scheduler_available(scheduler_path: &str, name: &str) -> bool {
    fs::read_to_string(scheduler_path)
        .map(|avail| avail.split_whitespace().any(|item| item.trim_matches(&['[', ']'][..]) == name))
        .unwrap_or(false)
}

impl IoManager {
    pub fn apply(&self, sched: IoScheduler, mode: &str) {
        // 目标调度器优先级列表（首选 → 回退）
        let candidates: Vec<(&str, &str, &str, &str, &str)> = match sched {
            IoScheduler::MqDeadline => vec![
                ("mq-deadline", "256", "128", "2", "0"),
                ("bfq",       "256", "128", "2", "0"),
                ("none",       "64",  "32", "0", "1"),
            ],
            IoScheduler::Deadline => vec![
                ("deadline",   "512", "256", "2", "1"),
                ("mq-deadline","256", "128", "2", "0"),
                ("bfq",        "256", "128", "2", "0"),
                ("none",       "64",  "32", "0", "1"),
            ],
            IoScheduler::Bfq => vec![
                ("bfq",        "256", "128", "2", "0"),
                ("mq-deadline","256", "128", "2", "0"),
                ("none",       "64",  "32", "0", "1"),
            ],
            IoScheduler::Noop => vec![
                ("none", "64", "32", "0", "1"),
            ],
        };

        for scheduler_path in real_queue_files() {
            // 从候选列表中找到第一个可用的调度器
            let chosen = candidates.iter().find(|(name, _, _, _, _)| {
                scheduler_available(&scheduler_path, name)
            });

            if let Some((name, ra_kb, nr_req, rq_af, nomerge)) = chosen {
                let _ = fs::write(&scheduler_path, name);
                if let Some(queue) = scheduler_path.strip_suffix("/scheduler") {
                    let values = [
                        ("read_ahead_kb", *ra_kb),
                        ("nr_requests", *nr_req),
                        ("rq_affinity", *rq_af),
                        ("nomerges", *nomerge),
                    ];
                    for (node, value) in values {
                        let _ = fs::write(format!("{}/{}", queue, node), value);
                    }
                    // 模式特定微调
                    if mode == "powersave" {
                        let _ = fs::write(format!("{}/read_ahead_kb", queue), "64");
                        let _ = fs::write(format!("{}/nr_requests", queue), "32");
                    }
                    if mode == "performance" {
                        let _ = fs::write(format!("{}/read_ahead_kb", queue), "1024");
                        let _ = fs::write(format!("{}/nr_requests", queue), "512");
                    }
                }
            }
        }
    }

    pub fn scheduler_for_mode(mode: &str) -> IoScheduler {
        match mode {
            "powersave" => IoScheduler::MqDeadline,
            "performance" => IoScheduler::MqDeadline,
            _ => IoScheduler::MqDeadline,
        }
    }

    pub fn current(&self) -> String {
        real_queue_files().into_iter().find_map(|path| fs::read_to_string(path).ok()).and_then(|s| {
            s.split_whitespace().find(|item| item.starts_with('[') && item.ends_with(']'))
                .map(|item| item.trim_matches(['[', ']']).to_string())
        }).unwrap_or_else(|| "none".to_string())
    }
}
