use std::fs;

const VM_SWAPPINESS: &str = "/proc/sys/vm/swappiness";
const VM_DIRTY_RATIO: &str = "/proc/sys/vm/dirty_ratio";
const VM_DIRTY_BG_RATIO: &str = "/proc/sys/vm/dirty_background_ratio";
const VM_DIRTY_WB_CS: &str = "/proc/sys/vm/dirty_writeback_centisecs";
const VM_DIRTY_EXPIRE: &str = "/proc/sys/vm/dirty_expire_centisecs";
const VM_VFS_CACHE: &str = "/proc/sys/vm/vfs_cache_pressure";
const VM_OVERCOMMIT: &str = "/proc/sys/vm/overcommit_memory";
const VM_DROP_CACHES: &str = "/proc/sys/vm/drop_caches";

pub struct VmManager;

impl VmManager {
    pub fn apply_mode(&self, mode: &str) {
        match mode {
            "powersave" => {
                // Aggressive swap to save RAM, low cache pressure for I/O
                let _ = fs::write(VM_SWAPPINESS, "60");
                let _ = fs::write(VM_DIRTY_RATIO, "20");
                let _ = fs::write(VM_DIRTY_BG_RATIO, "5");
                let _ = fs::write(VM_DIRTY_WB_CS, "1500");
                let _ = fs::write(VM_DIRTY_EXPIRE, "3000");
                let _ = fs::write(VM_VFS_CACHE, "100");
                let _ = fs::write(VM_OVERCOMMIT, "0");
            }
            "balance" => {
                // Balanced: moderate everything
                let _ = fs::write(VM_SWAPPINESS, "40");
                let _ = fs::write(VM_DIRTY_RATIO, "10");
                let _ = fs::write(VM_DIRTY_BG_RATIO, "5");
                let _ = fs::write(VM_DIRTY_WB_CS, "500");
                let _ = fs::write(VM_DIRTY_EXPIRE, "3000");
                let _ = fs::write(VM_VFS_CACHE, "60");
                let _ = fs::write(VM_OVERCOMMIT, "1");
            }
            "performance" => {
                // Low swappiness, high dirty ratio (buffer writes), low cache pressure
                let _ = fs::write(VM_SWAPPINESS, "10");
                let _ = fs::write(VM_DIRTY_RATIO, "40");
                let _ = fs::write(VM_DIRTY_BG_RATIO, "10");
                let _ = fs::write(VM_DIRTY_WB_CS, "3000");
                let _ = fs::write(VM_DIRTY_EXPIRE, "6000");
                let _ = fs::write(VM_VFS_CACHE, "30");
                let _ = fs::write(VM_OVERCOMMIT, "1");
            }
            _ => {}
        }
    }

    /// Drop page cache / inode cache (one-shot, useful during mode switch)
    pub fn drop_caches(&self) {
        let _ = fs::write(VM_DROP_CACHES, "3");
    }

    /// Read current swappiness
    pub fn current_swappiness(&self) -> u32 {
        fs::read_to_string(VM_SWAPPINESS)
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(60)
    }

    /// Read current cache pressure
    pub fn current_cache_pressure(&self) -> u32 {
        fs::read_to_string(VM_VFS_CACHE)
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(100)
    }
}
