use std::fs;
use std::process::Command;

const CGROUP_CPU: &str = "/dev/cpuctl";
const CGROUP_BG: &str = "/dev/cpuctl/background";
const CGROUP_FG: &str = "/dev/cpuctl/foreground";
const CGROUP_TOP: &str = "/dev/cpuctl/top-app";
const CGROUP_GAME: &str = "/dev/cpuctl/game";

pub struct CgroupManager {
    big_cores: Vec<u32>,
    little_cores: Vec<u32>,
}

impl CgroupManager {
    pub fn new(big_cores: &[u32], little_cores: &[u32]) -> Self {
        CgroupManager {
            big_cores: big_cores.to_vec(),
            little_cores: little_cores.to_vec(),
        }
    }

    /// Initialize cgroup structure if not present
    pub fn init(&self) {
        let _ = fs::create_dir_all(CGROUP_TOP);
        let _ = fs::create_dir_all(CGROUP_GAME);

        // Write available cpusets
        for (dir, cpus) in &[
            (CGROUP_TOP, self.all_cores_mask()),
            (CGROUP_GAME, self.big_cores_mask()),
        ] {
            let cpuset = format!("{}/cpuset.cpus", dir);
            let _ = fs::write(&cpuset, cpus);

            let mems = format!("{}/cpuset.mems", dir);
            let _ = fs::write(&mems, "0");
        }

        // Set default CPU shares
        let _ = fs::write(&format!("{}/cpu.shares", CGROUP_TOP), "1024");
        let _ = fs::write(&format!("{}/cpu.shares", CGROUP_GAME), "2048");
    }

    /// Assign game process to big-core cgroup
    pub fn assign_game(&self, pkg: &str) {
        let pids = self.get_pids(pkg);
        for pid in &pids {
            let tasks_file = format!("{}/tasks", CGROUP_GAME);
            let _ = fs::write(&tasks_file, pid);
            let _ = fs::write(&format!("{}/tasks", CGROUP_GAME), pid);
        }
    }

    /// Move process to foreground cgroup
    pub fn assign_foreground(&self, pkg: &str) {
        let pids = self.get_pids(pkg);
        for pid in &pids {
            let _ = fs::write(&format!("{}/tasks", CGROUP_FG), pid);
        }
    }

    /// Move process to background (little cores)
    pub fn assign_background(&self, pkg: &str) {
        let pids = self.get_pids(pkg);
        for pid in &pids {
            let _ = fs::write(&format!("{}/tasks", CGROUP_BG), pid);
        }
    }

    fn all_cores_mask(&self) -> String {
        let mask: u64 = self.little_cores.iter()
            .chain(self.big_cores.iter())
            .fold(0, |m, &c| m | (1u64 << c));
        mask_to_list(mask)
    }

    fn big_cores_mask(&self) -> String {
        let mask: u64 = self.big_cores.iter().fold(0, |m, &c| m | (1u64 << c));
        mask_to_list(mask)
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

fn mask_to_list(mask: u64) -> String {
    (0..64)
        .filter(|&i| (mask >> i) & 1 != 0)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
