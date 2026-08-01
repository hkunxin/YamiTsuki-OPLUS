use std::fs;

const IO_SCHED_BASE: &str = "/sys/block/mmcblk0/queue/scheduler";
const IO_SCHED_BASE2: &str = "/sys/block/sda/queue/scheduler";
const IO_SCHED_BASE3: &str = "/sys/block/nvme0n1/queue/scheduler";

const READ_AHEAD: &str = "/sys/block/mmcblk0/queue/read_ahead_kb";
const NR_REQUESTS: &str = "/sys/block/mmcblk0/queue/nr_requests";
const RQ_AFFINITY: &str = "/sys/block/mmcblk0/queue/rq_affinity";
const NOMERGES: &str = "/sys/block/mmcblk0/queue/nomerges";

pub enum IoScheduler {
    /// CFQ — 省电、低功耗
    Cfq,
    /// mq-deadline — 均衡
    MqDeadline,
    /// deadline — 游戏低延迟
    Deadline,
    /// Noop — 纯省电
    Noop,
}

pub struct IoManager;

impl IoManager {
    pub fn apply(&self, sched: IoScheduler, mode: &str) {
        let (name, ra_kb, nr_req, rq_af, nomerge) = match sched {
            IoScheduler::Cfq => ("cfq", "128", "64", "1", "0"),
            IoScheduler::MqDeadline => ("mq-deadline", "256", "128", "2", "0"),
            IoScheduler::Deadline => ("deadline", "512", "256", "2", "1"),
            IoScheduler::Noop => ("noop", "64", "32", "0", "1"),
        };

        for base in &[IO_SCHED_BASE, IO_SCHED_BASE2, IO_SCHED_BASE3] {
            if let Ok(avail) = fs::read_to_string(base) {
                let available_list = avail.trim().to_string();
                // Check if our target scheduler exists
                if available_list.contains(name) {
                    let _ = fs::write(base, name);
                }
            }
        }

        // Read-ahead, nr_requests, etc.
        let _ = fs::write(READ_AHEAD, ra_kb);
        let _ = fs::write(NR_REQUESTS, nr_req);
        let _ = fs::write(RQ_AFFINITY, rq_af);
        let _ = fs::write(NOMERGES, nomerge);

        // Per-mode additional tuning
        match mode {
            "powersave" => {
                let _ = fs::write(READ_AHEAD, "64");
                let _ = fs::write(NR_REQUESTS, "32");
            }
            "performance" => {
                let _ = fs::write(READ_AHEAD, "1024");
                let _ = fs::write(NR_REQUESTS, "512");
            }
            _ => {}
        }
    }

    pub fn scheduler_for_mode(mode: &str) -> IoScheduler {
        match mode {
            "powersave" => IoScheduler::Cfq,
            "performance" => IoScheduler::Deadline,
            _ => IoScheduler::MqDeadline,
        }
    }

    /// Return current active scheduler name
    pub fn current(&self) -> String {
        for base in &[IO_SCHED_BASE, IO_SCHED_BASE2, IO_SCHED_BASE3] {
            if let Ok(raw) = fs::read_to_string(base) {
                let s = raw.trim().to_string();
                // Format: "noop deadline [cfq]" → extract bracketed
                if let Some(start) = s.find('[') {
                    if let Some(end) = s.find(']') {
                        return s[start + 1..end].to_string();
                    }
                }
                // Or plain format
                return s;
            }
        }
        "none".to_string()
    }
}
