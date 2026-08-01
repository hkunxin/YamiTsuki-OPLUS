use std::fs;

const BLOCK_BASE: &str = "/sys/block";

pub enum IoScheduler { Cfq, MqDeadline, Deadline, Noop }
pub struct IoManager;

fn queue_files() -> Vec<String> {
    let Ok(entries) = fs::read_dir(BLOCK_BASE) else { return Vec::new(); };
    entries.filter_map(|entry| {
        let name = entry.ok()?.file_name().to_string_lossy().into_owned();
        let path = format!("{}/{}/queue/scheduler", BLOCK_BASE, name);
        fs::metadata(&path).ok().map(|_| path)
    }).collect()
}

impl IoManager {
    pub fn apply(&self, sched: IoScheduler, mode: &str) {
        let (name, ra_kb, nr_req, rq_af, nomerge) = match sched {
            IoScheduler::Cfq => ("cfq", "128", "64", "1", "0"),
            IoScheduler::MqDeadline => ("mq-deadline", "256", "128", "2", "0"),
            IoScheduler::Deadline => ("deadline", "512", "256", "2", "1"),
            IoScheduler::Noop => ("noop", "64", "32", "0", "1"),
        };
        for scheduler in queue_files() {
            if let Ok(avail) = fs::read_to_string(&scheduler) {
                if avail.split_whitespace().any(|item| item.trim_matches(['[', ']']) == name) {
                    let _ = fs::write(&scheduler, name);
                }
                if let Some(queue) = scheduler.strip_suffix("/scheduler") {
                    let values = [("read_ahead_kb", ra_kb), ("nr_requests", nr_req), ("rq_affinity", rq_af), ("nomerges", nomerge)];
                    for (node, value) in values { let _ = fs::write(format!("{}/{}", queue, node), value); }
                    if mode == "powersave" { let _ = fs::write(format!("{}/read_ahead_kb", queue), "64"); let _ = fs::write(format!("{}/nr_requests", queue), "32"); }
                    if mode == "performance" { let _ = fs::write(format!("{}/read_ahead_kb", queue), "1024"); let _ = fs::write(format!("{}/nr_requests", queue), "512"); }
                }
            }
        }
    }

    pub fn scheduler_for_mode(mode: &str) -> IoScheduler {
        match mode { "powersave" => IoScheduler::Cfq, "performance" => IoScheduler::Deadline, _ => IoScheduler::MqDeadline }
    }

    pub fn current(&self) -> String {
        queue_files().into_iter().find_map(|path| fs::read_to_string(path).ok()).and_then(|s| {
            s.split_whitespace().find(|item| item.starts_with('[') && item.ends_with(']')).map(|item| item.trim_matches(['[', ']']).to_string())
        }).unwrap_or_else(|| "none".to_string())
    }
}
