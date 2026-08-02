use std::fs;
use std::process::Command;

pub struct ThreadOptimizer;

impl ThreadOptimizer {
    pub fn new(_big_cores: &[u32]) -> Self {
        ThreadOptimizer
    }

    pub fn optimize_game(&self, pkg: &str) -> usize {
        self.optimize_game_with_policy(pkg, true, true)
    }

    pub fn optimize_game_with_policy(&self, pkg: &str, _allow_affinity: bool, _allow_realtime: bool) -> usize {
        let _ = self.get_threads(pkg);
        0
    }

    pub fn restore_game(&self, pkg: &str) {
        let _ = self.get_threads(pkg);
    }

    fn get_threads(&self, pkg: &str) -> Vec<String> {
        let mut tids = vec![];
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
