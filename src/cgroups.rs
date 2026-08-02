use std::fs;
use std::path::Path;
use std::process::Command;

const CGROUP_BG: &str = "/dev/cpuctl/background";
const CGROUP_FG: &str = "/dev/cpuctl/foreground";
const CGROUP_TOP: &str = "/dev/cpuctl/top-app";
const CGROUP_GAME: &str = "/dev/cpuctl/game";

pub struct CgroupManager;

impl CgroupManager {
    pub fn new(_big_cores: &[u32], _little_cores: &[u32]) -> Self {
        CgroupManager
    }

    pub fn init(&self) {
    }

    pub fn assign_game(&self, pkg: &str) {
        self.assign_to_group(CGROUP_GAME, pkg);
    }

    pub fn assign_foreground(&self, pkg: &str) {
        self.assign_to_group(CGROUP_FG, pkg);
    }

    pub fn assign_background(&self, pkg: &str) {
        self.assign_to_group(CGROUP_BG, pkg);
    }

    fn assign_to_group(&self, group: &str, pkg: &str) {
        let procs = format!("{}/cgroup.procs", group);
        let tasks = format!("{}/tasks", group);
        let target = if Path::new(&procs).exists() {
            procs
        } else if Path::new(&tasks).exists() {
            tasks
        } else {
            return;
        };
        for pid in self.get_pids(pkg) {
            let _ = fs::write(&target, pid);
        }
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
