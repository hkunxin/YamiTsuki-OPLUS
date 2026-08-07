use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::process::Command;

const CGROUP_GAME: &str = "/dev/cpuctl/game";

pub struct CgroupManager {
    successful: AtomicUsize,
    failed: AtomicUsize,
}

impl CgroupManager {
    pub fn new(_big_cores: &[u32], _little_cores: &[u32]) -> Self {
        CgroupManager { successful: AtomicUsize::new(0), failed: AtomicUsize::new(0) }
    }

    pub fn init(&self) {
    }

    pub fn assign_game(&self, pkg: &str) -> usize {
        self.assign_to_group(CGROUP_GAME, pkg)
    }

    pub fn diagnostic_counts(&self) -> (usize, usize) {
        (self.successful.load(Ordering::Relaxed), self.failed.load(Ordering::Relaxed))
    }


    fn assign_to_group(&self, group: &str, pkg: &str) -> usize {
        let procs = format!("{}/cgroup.procs", group);
        let tasks = format!("{}/tasks", group);
        let target = if Path::new(&procs).exists() {
            procs
        } else if Path::new(&tasks).exists() {
            tasks
        } else {
            self.failed.fetch_add(1, Ordering::Relaxed);
            return 0;
        };
        let mut moved = 0;
        for pid in self.get_pids(pkg) {
            let pid_path = format!("/proc/{}/cgroup", pid);
            let group_name = group.rsplit('/').next().unwrap_or(group);
            let ok = fs::write(&target, &pid).is_ok()
                && fs::read_to_string(&pid_path).map(|raw| raw.lines().any(|line| line.ends_with(&format!("/{}", group_name)) || line.ends_with(group))).unwrap_or(false);
            if ok {
                moved += 1;
                self.successful.fetch_add(1, Ordering::Relaxed);
            } else {
                self.failed.fetch_add(1, Ordering::Relaxed);
            }
        }
        moved
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
