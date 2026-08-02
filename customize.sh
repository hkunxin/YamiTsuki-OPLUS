#!/system/bin/sh

ui_print "🌙 YamiTsuki-OPLUS 模块 V2.0 安装中..."
ui_print "🔧 正在部署..."

OLD_MODULE="/data/adb/modules/yamitsuki_oplus"
IS_UPDATE=0
if [ -d "$OLD_MODULE" ]; then
    IS_UPDATE=1
    ui_print "🔄 检测到旧版本，将沿用你的配置"
fi

FEATURES="charge_boost_enabled horae_enabled hw_overlay_enabled step_charging_enabled mt_hide_enabled prop_enabled disable_usb_enabled"
ENABLED_FEATURES=""

if [ "$IS_UPDATE" -eq 1 ]; then
    ui_print "📂 读取旧配置..."
    if [ -f "$OLD_MODULE/log_config.conf" ]; then
        cp -f "$OLD_MODULE/log_config.conf" "$MODPATH/log_config.conf" 2>/dev/null || true
        ui_print "  ✅ 沿用: log_config.conf"
    fi
    for f in $FEATURES; do
        if [ -f "$OLD_MODULE/$f" ]; then
            ENABLED_FEATURES="$ENABLED_FEATURES $f"
            ui_print "  ✅ 沿用: $f"
        else
            ui_print "  ⚪ 未启用: $f"
        fi
    done
else
    ui_print "⚡ 首次安装，全部功能默认启用"
    ENABLED_FEATURES=$FEATURES
fi

ui_print "📂 创建目录..."
mkdir -p $MODPATH/webroot
mkdir -p $MODPATH/bin

ui_print "📄 复制文件..."
[ -f "$MODPATH/service.sh" ] && cp -f "$MODPATH/service.sh" "$MODPATH/" || true
[ -f "$MODPATH/game_list.txt" ] && cp -f "$MODPATH/game_list.txt" "$MODPATH/" || true
if [ -f "$MODPATH/bin/yamitsuki_rs" ]; then
    cp -f "$MODPATH/bin/yamitsuki_rs" "$MODPATH/bin/" || true
else
    ui_print "  ⚠️ yamitsuki_rs 不存在，跳过"
fi
[ -f "$MODPATH/webroot/index.html" ] && cp -f "$MODPATH/webroot/index.html" "$MODPATH/webroot/" || true
[ -f "$MODPATH/webroot/style.css" ] && cp -f "$MODPATH/webroot/style.css" "$MODPATH/webroot/" || true
[ -f "$MODPATH/webroot/script.js" ] && cp -f "$MODPATH/webroot/script.js" "$MODPATH/webroot/" || true

set_perm_recursive $MODPATH 0 0 0755 0755 2>/dev/null || true
set_perm $MODPATH/game_list.txt 0 0 0644 2>/dev/null || true
[ -f "$MODPATH/bin/yamitsuki_rs" ] && set_perm $MODPATH/bin/yamitsuki_rs 0 0 0755 2>/dev/null || true

ui_print "⚡ 创建功能标志..."
for f in $FEATURES; do
    if echo "$ENABLED_FEATURES" | grep -q "$f"; then
        touch "$MODPATH/$f"
        ui_print "  ✅ 启用: $f"
    else
        [ -f "$MODPATH/$f" ] && rm -f "$MODPATH/$f"
        ui_print "  ⚪ 跳过: $f"
    fi
done

echo "auto" > /data/local/tmp/yamitsuki_mode 2>/dev/null || true
chmod 644 /data/local/tmp/yamitsuki_mode 2>/dev/null || true

touch "$MODPATH/yamitsuki.log" 2>/dev/null || true
chmod 644 "$MODPATH/yamitsuki.log" 2>/dev/null || true
if [ "$IS_UPDATE" -eq 1 ]; then
    echo "[$(date)] YamiTsuki 已更新" >> "$MODPATH/yamitsuki.log" 2>/dev/null || true
else
    echo "[$(date)] YamiTsuki 已安装" >> "$MODPATH/yamitsuki.log" 2>/dev/null || true
fi

ui_print ""
ui_print "╔════════════════════════════════════════════════════════════╗"
ui_print "║   🎉 YamiTsuki 安装/更新完成！                           ║"
ui_print "║   🌙 重启后生效                                          ║"
ui_print "║   🐧 有问题？加入炒饭社让大佬们狠狠嘲笑你吧！            ║"
ui_print "║   🔗 https://qm.qq.com/q/81PdbfU7f2                      ║"
ui_print "╚════════════════════════════════════════════════════════════╝"
ui_print ""

sleep 1

ui_print "🌀 正在跳转炒饭社…… (如果没反应，杂鱼前辈就手动加吧)"
if command -v am >/dev/null 2>&1; then
    am start -a android.intent.action.VIEW -d "https://qm.qq.com/q/81PdbfU7f2" >/dev/null 2>&1 &
else
    ui_print "⚠️ 无法启动跳转，am 命令不存在"
fi

ui_print ""
ui_print "  🌙 月之暗面与你同在，杂鱼前辈。"
ui_print ""

exit 0