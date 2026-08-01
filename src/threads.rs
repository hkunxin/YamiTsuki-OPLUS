use std::fs;
use std::process::Command;

/// Thread optimization module:
/// - Bind game rendering threads to big cores
/// - Set SCHED_FIFO real-time priority for critical threads
pub struct ThreadOptimizer {
    big_cores: Vec<u32>,
}

impl ThreadOptimizer {
    pub fn new(big_cores: &[u32]) -> Self {
        ThreadOptimizer {
            big_cores: big_cores.to_vec(),
        }
    }

    /// Apply thread optimization for a given game package
    pub fn optimize_game(&self, pkg: &str) -> usize {
        if self.big_cores.is_empty() {
            return 0;
        }

        // Get all threads (TIDs) of the game process
        let tids = self.get_threads(pkg);
        let mut count = 0;

        for tid in &tids {
            let tid_file = format!("/proc/{}/task/{}/comm", tid, tid);
            let comm = fs::read_to_string(&tid_file).unwrap_or_default().trim().to_string();

            // Prioritize rendering and important threads
            let is_critical = comm.contains("RenderThread")
                || comm.contains("GameThread")
                || comm.contains("GLThread")
                || comm.contains("UnityMain")
                || comm.contains("Choreographer")
                || comm.contains("hwuiTask");

            if is_critical {
                // Bind to first big core
                if let Some(&core) = self.big_cores.first() {
                    let affinity_path = format!("/proc/{}/task/{}/cpus_allowed", tid, tid);
                    let mask = 1u64 << core;
                    let _ = fs::write(&affinity_path, format!("{}", mask));
                }

                // Set SCHED_FIFO priority 50 (range 1-99)
                let sched_path = format!("/proc/{}/task/{}/sched", tid, tid);
                let prio_path = format!("/proc/{}/task/{}/sched_priority", tid, tid);
                let policy_path = format!("/proc/{}/task/{}/sched_policy", tid, tid);

                let _ = fs::write(&policy_path, "1");  // SCHED_FIFO
                let _ = fs::write(&prio_path, "50");
                count += 1;
            } else {
                // Non-critical threads → affinity to big cores for performance
                let mask: u64 = self.big_cores.iter().fold(0, |m, &c| m | (1u64 << c));
                let affinity_path = format!("/proc/{}/task/{}/cpus_allowed", tid, tid);
                let _ = fs::write(&affinity_path, format!("{}", mask));
            }
        }

        count
    }

    /// Restore threads from SCHED_FIFO back to SCHED_NORMAL
    pub fn restore_game(&self, pkg: &str) {
        let tids = self.get_threads(pkg);
        for tid in &tids {
            let policy_path = format!("/proc/{}/task/{}/sched_policy", tid, tid);
            let _ = fs::write(&policy_path, "0"); // SCHED_NORMAL
        }
    }

    fn get_threads(&self, pkg: &str) -> Vec<String> {
        let mut tids = vec![];

        // Try via /proc/[pid]/task
        for pid in self.get_pids(pkg) {
            let task_dir = format!("/proc/{}/task", pid);
            if let Ok(entries) = fs::read_dir(&task_dir) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.chars().all(|c| c.is_ascii_digit()) {
                            tids.push(name);
                        }
                    }
                }
            }
        }

        tids
    }

    fn get_pids(&self, pkg: &str) -> Vec<String> {
        if let Ok(output) = Command::new("pidof").arg(pkg).output() {
            let raw = String::from_utf8_lossy(&output.stdout);
            raw.trim().split_whitespace().map(|s| s.to_string()).collect()
        } else {
            vec![]
        }
    }
}
