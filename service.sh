#!/system/bin/sh

MODDIR=${0%/*}
LOG_FILE="$MODDIR/yamitsuki.log"
MODE_FILE="/data/local/tmp/yamitsuki_mode"
CMD_FILE="/data/local/tmp/yamitsuki_cmd"
GAME_LIST="$MODDIR/game_list.txt"
DAEMON="$MODDIR/bin/yamitsuki_rs"

echo "[$(date)] YamiTsuki V2.0 服务启动" >> $LOG_FILE

BRAND=$(getprop ro.product.brand)
if ! echo "$BRAND" | grep -qiE "OPPO|OnePlus|realme"; then
    echo "[$(date)] ❌ 非欧加真设备" >> $LOG_FILE
    exit 1
fi

[ ! -f "$GAME_LIST" ] && {
    echo "com.tencent.tmgp.sgame" > $GAME_LIST
    echo "com.miHoYo.GenshinImpact" >> $GAME_LIST
}
[ ! -f "$MODE_FILE" ] && echo "auto" > $MODE_FILE
[ ! -f "$CMD_FILE" ] && echo "" > $CMD_FILE

if [ -f "$DAEMON" ] && [ -x "$DAEMON" ]; then
    echo "[$(date)] 🚀 启动 Rust 底层引擎..." >> $LOG_FILE
    pkill -f "yamitsuki_rs" 2>/dev/null
    nohup "$DAEMON" >/dev/null 2>&1 &
fi

echo "[$(date)] 引擎已启动，调度由内部 Scheduler 负责" >> $LOG_FILE

sleep 1