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
        let config = crate::config::load(mode);
        let write = |path: &str, value: u32| {
            let _ = fs::write(path, value.to_string());
        };
        write(VM_SWAPPINESS, config.vm_swappiness);
        write(VM_DIRTY_RATIO, config.vm_dirty_ratio);
        write(VM_DIRTY_BG_RATIO, config.vm_dirty_background_ratio);
        write(VM_DIRTY_WB_CS, config.vm_dirty_writeback);
        write(VM_DIRTY_EXPIRE, config.vm_dirty_expire);
        write(VM_VFS_CACHE, config.vm_vfs_cache);
        write(VM_OVERCOMMIT, config.vm_overcommit);

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
