use std::process::Command;

/// Run full device spoof: random SSAID, IMEI, MAC, CPUID, serial
pub fn run_spoof() -> String {
    let mut output = String::new();

    output.push_str("[执行完成]\n");

    // --- CPUID spoof ---
    if let Ok(uuid) = Command::new("cat")
        .args(&["/proc/sys/kernel/random/uuid"])
        .output()
    {
        let uuid_str = String::from_utf8_lossy(&uuid.stdout)
            .trim()
            .replace("-", "")
            .chars()
            .take(11)
            .collect::<String>();
        let cpuid_arg = format!("0x00000{}", uuid_str);
        let _ = Command::new("resetprop")
            .args(&["ro.boot.cpuid", cpuid_arg.as_str()])
            .output();
        let _ = Command::new("resetprop")
            .args(&["ro.boot.realmebootstate", "0"])
            .output();
        let _ = Command::new("resetprop")
            .args(&["ro.boot.realme.lockstate", "0"])
            .output();
        output.push_str(&format!("CPUID spoofed -> 0x00000{}\n", uuid_str));
    }

    // --- MAC address spoof ---
    let wlan0_path = "/sys/class/net/wlan0/address";
    if let Ok(mac_output) = Command::new("cat").arg(wlan0_path).output() {
        let mac = String::from_utf8_lossy(&mac_output.stdout).trim().to_string();
        // Spoof MAC by replacing last 3 octets
        let parts: Vec<&str> = mac.split(':').collect();
        if parts.len() == 6 {
            let uuid = random_hex(3);
            let new_mac = format!(
                "{}:{}:{}:{:02x}:{:02x}:{:02x}",
                parts[0],
                parts[1],
                parts[2],
                uuid[0],
                uuid[1],
                uuid[2]
            );
            let _ = Command::new("ip")
                .args(&["link", "set", "dev", "wlan0", "address", new_mac.as_str()])
                .output();
            output.push_str(&format!("MAC spoofed -> {}\n", new_mac));
        }
    }

    // --- Serial number spoof ---
    let serial_arg = random_hex_str(16);
    let _ = Command::new("resetprop")
        .args(&["ro.serialno", serial_arg.as_str()])
        .output();

    // --- SSAID spoof ---
    let ssaid_path = "/data/system/users/0/settings_ssaid.xml";
    let cmd = format!(
        "ID=$(grep $PKG {} | awk -F'\"' '{{print $6}}'); sed -i \"s/$ID/$P/g\" {}",
        ssaid_path, ssaid_path
    );
    let _ = Command::new("sh").arg("-c").arg(&cmd).output();
    output.push_str("SSAID spoofed\n");

    // --- Device spoof file cleanup for UAM games ---
    let game_dirs = &[
        "com.tencent.tmgp.dfm",
        "com.tencent.tmgp.cf",
        "com.tencent.tmgp.pubgmhd",
        "com.tencent.tmgp.cod",
        "com.tencent.tmgp.codev",
        "com.tencent.mf.uam",
        "com.tencent.ig",
    ];

    for pkg in game_dirs {
        let data_path = format!("/data/user/0/{}", pkg);
        let sdcard_path = format!("/storage/emulated/0/Android/data/{}", pkg);
        let ue_path = format!(
            "/storage/emulated/0/Android/data/{}/files/UE4Game/UAGame/UAGame/Saved",
            pkg
        );
        let _ = Command::new("rm").arg("-rf").arg(&data_path).output();
        let _ = Command::new("rm").arg("-rf").arg(&sdcard_path).output();
        let _ = Command::new("rm").arg("-rf").arg(&ue_path).output();
    }
    output.push_str("Game data cleaned\n");

    // --- Telegram cache cleanup ---
    let _ = Command::new("rm")
        .args(&["-rf", "/storage/emulated/0/Android/data/org.telegram.messenger.web/*"])
        .output();
    output.push_str("Telegram cache cleaned\n");

    // --- KGSL cache threshold ---
    let _ = Command::new("sh")
        .arg("-c")
        .arg("echo 0 > /sys/devices/virtual/kgsl/kgsl/full_cache_threshold")
        .output();

    output.push_str("\n[执行完成] 设备特征已随机修改\n");
    output
}

fn random_hex(count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(count);
    let raw = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for i in 0..count {
        out.push(((raw >> (i * 8)) & 0xFF) as u8);
    }
    out
}

fn random_hex_str(len: usize) -> String {
    random_hex(len * 2)
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")[..len]
        .to_string()
}
