#!/system/bin/sh
# 应用名解析诊断脚本：在设备上实测各候选命令，取你验证可用者反馈。
# 用法：sh resolve_label_test.sh  com.android.settings [包名2 ...]
#      不传包名时默认测试 com.android.settings / com.tencent.mm(若存在)

PKG="com.android.settings"
if [ -n "$1" ]; then
    PKG="$1"
fi

echo "===== 目标包名: $PKG ====="
echo

echo "[1] dumpsys package 直接 grep label 相关字段"
dumpsys package "$PKG" 2>/dev/null | grep -iE "ApplicationLabel|nonLocalizedLabel|label=" | head -5
echo

echo "[2] dumpsys package first-partyMainApp / 其他 label 键"
dumpsys package "$PKG" 2>/dev/null | grep -iE "label|Label" | head -5 | sed 's/^[[:space:]]*//'
echo

echo "[3] cmd package resolve-activity --brief（裸输出）"
cmd package resolve-activity --brief "$PKG" 2>/dev/null | tail -n 2
echo

echo "[4] aapt/aapt2 遍历探测 application-label（改进后逻辑）"
for bin in aapt aapt2; do
    P=$(command -v "$bin" 2>/dev/null)
    [ -n "$P" ] || { echo "  $bin: 未在 PATH 找到"; continue; }
    apk=$(pm path "$PKG" 2>/dev/null | head -1 | sed 's/^package://')
    echo "  $bin: $P"
    echo "  APK: $apk"
    "$bin" dump badging "$apk" 2>/dev/null | grep -m1 '^application-label:'
    break
done
echo

echo "[5] pm list packages -f 拿到 APK 路径（供后续解析）"
pm path "$PKG" 2>/dev/null
echo

echo "[6] dumpsys package 中 Package [..] 下一行的 versionName 前 label（结构化区）"
dumpsys package "$PKG" 2>/dev/null | grep -m1 -B1 'versionName=' 
echo

echo "[7] 备选：resolve-activity 非 brief（长格式，可能含 label）"
cmd package resolve-activity "$PKG" 2>/dev/null | grep -iE 'label|mName|result=0|0x[0-9a-f]{8}' | head -6
echo

echo "[8] dumpsys window 当前前台（验证包名采集源）"
dumpsys window 2>/dev/null | grep -E 'mCurrentFocus|mFocusedApp' | head -1
echo
echo "===== 测试结束 ====="