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
        let config = crate::config::load(mode);
        // 目标调度器优先级列表（首选 → 回退）
        let configured_scheduler = config.io_scheduler.as_str().trim();
        let candidates = match sched {
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
                *name == configured_scheduler && scheduler_available(&scheduler_path, name)
            }).or_else(|| candidates.iter().find(|(name, _, _, _, _)| scheduler_available(&scheduler_path, name)));

            if let Some((name, _ra_kb, _nr_req, _rq_af, _nomerge)) = chosen {
                let _ = fs::write(&scheduler_path, name);
                if let Some(queue) = scheduler_path.strip_suffix("/scheduler") {
                    let values = [
                        ("read_ahead_kb", config.io_read_ahead.to_string()),
                        ("nr_requests", config.io_nr_requests.to_string()),
                        ("rq_affinity", config.io_rq_affinity.to_string()),
                        ("nomerges", config.io_nomerges.to_string()),
                    ];
                    for (node, value) in values {
                        let _ = fs::write(format!("{}/{}", queue, node), value);
                    }

                }
            }
        }
    }

    pub fn scheduler_for_mode(mode: &str) -> IoScheduler {
        let config = crate::config::load(mode);
        match config.io_scheduler.as_str() {
            "bfq" => IoScheduler::Bfq,
            "deadline" => IoScheduler::Deadline,
            "noop" | "none" => IoScheduler::Noop,
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
