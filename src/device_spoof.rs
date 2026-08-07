use std::fs;
use std::process::Command;

/// Run full device spoof: random CPUID, MAC, serial
pub fn run_spoof() -> String {
    let mut output = String::new();
    output.push_str("[执行开始]\n");

    // --- CPUID spoof ---
    if let Ok(uuid) = Command::new("cat").args(["/proc/sys/kernel/random/uuid"]).output() {
        let uuid_str = String::from_utf8_lossy(&uuid.stdout).trim().replace("-", "").chars().take(11).collect::<String>();
        let cpuid_arg = format!("0x00000{}", uuid_str);
        let ok1 = Command::new("resetprop").args(["ro.boot.cpuid", cpuid_arg.as_str()]).output().map(|o| o.status.success()).unwrap_or(false);
        let ok2 = Command::new("resetprop").args(["ro.boot.realmebootstate", "0"]).output().map(|o| o.status.success()).unwrap_or(false);
        let ok3 = Command::new("resetprop").args(["ro.boot.realme.lockstate", "0"]).output().map(|o| o.status.success()).unwrap_or(false);
        if ok1 && ok2 && ok3 {
            output.push_str(&format!("CPUID spoofed -> 0x00000{}\n", uuid_str));
        } else {
            output.push_str("CPUID spoof failed (resetprop unavailable?)\n");
        }
    } else {
        output.push_str("CPUID spoof failed (cannot read uuid)\n");
    }

    // --- MAC address spoof ---
    let wlan0_path = "/sys/class/net/wlan0/address";
    if let Ok(mac_output) = Command::new("cat").arg(wlan0_path).output() {
        let mac = String::from_utf8_lossy(&mac_output.stdout).trim().to_string();
        let parts: Vec<&str> = mac.split(':').collect();
        if parts.len() == 6 {
            let rand = random_hex(3);
            let new_mac = format!("{}:{}:{}:{:02x}:{:02x}:{:02x}", parts[0], parts[1], parts[2], rand[0], rand[1], rand[2]);
            let mac_ok = Command::new("ip").args(["link", "set", "dev", "wlan0", "address", new_mac.as_str()]).output().map(|o| o.status.success()).unwrap_or(false);
            if mac_ok {
                output.push_str(&format!("MAC spoofed -> {}\n", new_mac));
            } else {
                output.push_str("MAC spoof failed (ip command error)\n");
            }
        } else {
            output.push_str("MAC spoof skipped (invalid original MAC)\n");
        }
    } else {
        output.push_str("MAC spoof skipped (wlan0 not found)\n");
    }

    // --- Serial number spoof ---
    let serial_arg = random_hex_str(16);
    let serial_ok = Command::new("resetprop").args(["ro.serialno", serial_arg.as_str()]).output().map(|o| o.status.success()).unwrap_or(false);
    if serial_ok {
        output.push_str(&format!("Serial spoofed -> {}\n", serial_arg));
    } else {
        output.push_str("Serial spoof failed\n");
    }

    output.push_str("SSAID unchanged (requires per-app, explicit action)\n");
    output.push_str("Game and Telegram data were not deleted\n");

    // --- KGSL cache threshold ---
    let kgsl_ok = Command::new("sh").arg("-c").arg("echo 0 > /sys/devices/virtual/kgsl/kgsl/full_cache_threshold").output().map(|o| o.status.success()).unwrap_or(false);
    if kgsl_ok {
        output.push_str("KGSL cache threshold set\n");
    } else {
        output.push_str("KGSL cache threshold failed (node may not exist)\n");
    }

    output.push_str("\n[执行完成] 设备特征已随机修改\n");
    output
}

fn random_hex(count: usize) -> Vec<u8> {
    let mut out = vec![0u8; count];
    if fs::File::open("/dev/urandom").and_then(|mut f| std::io::Read::read_exact(&mut f, &mut out)).is_err() {
        return vec![0; count];
    }
    out
}

fn random_hex_str(len: usize) -> String {
    random_hex(len * 2).iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join("")[..len].to_string()
}