#!/system/bin/sh
# YamiTsuki-OPLUS V2.0 卸载脚本

MODDIR=${0%/*}
LOG_FILE="$MODDIR/yamitsuki.log"

echo "[$(date)] YamiTsuki 卸载开始" >> "$LOG_FILE" 2>/dev/null

# 1. 杀掉守护进程
echo "[$(date)] 停止引擎..." >> "$LOG_FILE" 2>/dev/null
pkill -f "yamitsuki_rs" 2>/dev/null
sleep 1

# 2. 恢复 CPU 默认设置
echo "[$(date)] 恢复 CPU 频率限制..." >> "$LOG_FILE" 2>/dev/null
for cpu in 0 1 2 3 4 5 6 7; do
    MAX_PATH="/sys/devices/system/cpu/cpu${cpu}/cpufreq/cpuinfo_max_freq"
    TARGET="/sys/devices/system/cpu/cpu${cpu}/cpufreq/scaling_max_freq"
    GOVERNOR="/sys/devices/system/cpu/cpu${cpu}/cpufreq/scaling_governor"
    if [ -f "$MAX_PATH" ] && [ -f "$TARGET" ]; then
        cat "$MAX_PATH" > "$TARGET" 2>/dev/null
    fi
    if [ -f "$GOVERNOR" ]; then
        echo "schedutil" > "$GOVERNOR" 2>/dev/null
    fi
done

# 3. 恢复 GPU 默认
echo "[$(date)] 恢复 GPU 频率..." >> "$LOG_FILE" 2>/dev/null
GPU_MAX="/sys/class/kgsl/kgsl-3d0/gpu_available_frequencies"
GPU_TARGET="/sys/class/kgsl/kgsl-3d0/max_gpuclk"
GPU_RAIL="/sys/class/kgsl/kgsl-3d0/force_rail_on"
if [ -f "$GPU_MAX" ]; then
    # 取最高频率恢复
    MAX_GPU=$(cat "$GPU_MAX" 2>/dev/null | tr ' ' '\n' | sort -rn | head -1)
    [ -n "$MAX_GPU" ] && echo "$MAX_GPU" > "$GPU_TARGET" 2>/dev/null
fi
[ -f "$GPU_RAIL" ] && echo "0" > "$GPU_RAIL" 2>/dev/null

# 4. 恢复充电电流
CHARGE_PATH="/sys/class/power_supply/battery/constant_charge_current"
[ -f "$CHARGE_PATH" ] && echo "3000000" > "$CHARGE_PATH" 2>/dev/null

# 5. 恢复 Horae
setprop persist.sys.horae.enable 1 2>/dev/null

# 6. 恢复 USB 调试
settings put global adb_enabled 1 2>/dev/null
settings put global development_settings_enabled 1 2>/dev/null

# 7. 恢复阶梯充电
STEP_PATH="/sys/class/power_supply/battery/step_charging_enabled"
[ -f "$STEP_PATH" ] && echo "1" > "$STEP_PATH" 2>/dev/null

# 8. 卸载 MT 管理器绑定
for pkg in bin.mt.plus bin.mt.plus.canary bin.mt.plus.pro bin.mt.plus.mod bin.mt.plus.mtz; do
    umount -l "/data/data/$pkg" 2>/dev/null
done
rm -rf /dev/fk_bypass_empty 2>/dev/null

# 9. 恢复属性
for prop in $(getprop | grep -E 'yamitsuki|persist.sys.yamitsuki' | cut -d: -f1); do
    resetprop -d "$prop" 2>/dev/null
done

# 10. 清理运行时文件
rm -f /data/local/tmp/yamitsuki_mode 2>/dev/null
rm -f /data/local/tmp/yamitsuki_cmd 2>/dev/null
rm -f /data/local/tmp/governor_info 2>/dev/null
rm -f /data/adb/modules/yamitsuki_oplus/governor_selected 2>/dev/null
rm -f /data/adb/modules/yamitsuki_oplus/device_spoof_result.txt 2>/dev/null

# 11. 清理 cron / 定时任务
crond -c /data/adb/modules/yamitsuki_oplus/crontab -K 2>/dev/null

echo "[$(date)] YamiTsuki 卸载完成" >> "$LOG_FILE" 2>/dev/null
echo "YamiTsuki V2.0 已卸载。重启手机即可彻底移除。"