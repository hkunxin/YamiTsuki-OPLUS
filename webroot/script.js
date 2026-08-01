// ============================================================
// YamiTsuki WebUI V2.0 — 完整控制脚本
// ============================================================

var _execCallbackId = 0;

function execCommand(cmd, timeout) {
    timeout = timeout || 10000;
    return new Promise(function(resolve) {
        var cbName = "cb_" + Date.now() + "_" + (++_execCallbackId);
        var timer = setTimeout(function() {
            delete window[cbName];
            resolve({ errno: -1, stdout: "", stderr: "timeout" });
        }, timeout);

        window[cbName] = function(errno, stdout, stderr) {
            clearTimeout(timer);
            delete window[cbName];
            resolve({
                errno: Number(errno) || 0,
                stdout: stdout || "",
                stderr: stderr || ""
            });
        };

        try {
            if (typeof ksu !== "undefined" && ksu.exec) {
                ksu.exec(cmd, "{}", cbName);
            } else {
                clearTimeout(timer);
                delete window[cbName];
                resolve({ errno: -1, stdout: "", stderr: "No exec interface" });
            }
        } catch (e) {
            clearTimeout(timer);
            delete window[cbName];
            resolve({ errno: -1, stdout: "", stderr: String(e) });
        }
    });
}

var toastTimer = null;

function showToast(msg, type) {
    type = type || "success";
    var el = document.getElementById("toast");
    var icons = { success: "✅", info: "ℹ️", warning: "⚠️", error: "❌" };
    el.className = "toast " + type;
    el.querySelector(".toast-icon").textContent = icons[type] || "✅";
    el.querySelector(".toast-msg").textContent = msg;
    el.classList.add("show");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function() {
        el.classList.remove("show");
    }, 2800);
}

var MODULE_DIR = "/data/adb/modules/yamitsuki_oplus";
var MODE_FILE = "/data/local/tmp/yamitsuki_mode";
var CMD_FILE = "/data/local/tmp/yamitsuki_cmd";
var GAME_LIST = MODULE_DIR + "/game_list.txt";
var LOG_FILE = MODULE_DIR + "/yamitsuki.log";

var FEATURE_KEYS = {
    'charge_boost': 'yamitsuki_charge_boost',
    'horae': 'yamitsuki_horae',
    'hw_overlay': 'yamitsuki_hw_overlay',
    'step_charging': 'yamitsuki_step_charging',
    'mt_hide': 'yamitsuki_mt_hide',
    'prop': 'yamitsuki_prop',
    'disable_usb': 'yamitsuki_disable_usb'
};

var FEATURE_CMDS = {
    'charge_boost': { enable: 'charge_boost:enable', disable: 'charge_boost:disable' },
    'horae': { enable: 'horae:enable', disable: 'horae:disable' },
    'hw_overlay': { enable: 'hw_overlay:enable', disable: 'hw_overlay:disable' },
    'step_charging': { enable: 'step_charging:enable', disable: 'step_charging:disable' },
    'mt_hide': { enable: 'mt_hide:enable', disable: 'mt_hide:disable' },
    'prop': { enable: 'prop:enable', disable: 'prop:disable' },
    'disable_usb': { enable: 'disable_usb:enable', disable: 'disable_usb:disable' }
};

// ============================================================
// 页面切换
// ============================================================
function switchTab(tab) {
    document.querySelectorAll('.tab-content').forEach(function(el) {
        el.classList.remove('active');
    });
    document.querySelectorAll('.nav-tab').forEach(function(el) {
        el.classList.remove('active');
    });
    document.getElementById('tab-' + tab).classList.add('active');
    document.querySelector('.nav-tab[data-tab="' + tab + '"]').classList.add('active');
    if (tab === 'features') {
        restoreAllStates();
    }
    if (tab === 'main') {
        setTimeout(function() { loadGovernors(); }, 300);
    }
}

// ============================================================
// 模式切换
// ============================================================
async function setMode(mode) {
    document.querySelectorAll(".mode-btn").forEach(function(b) {
        b.classList.remove("active");
    });
    var btn = document.querySelector('.mode-btn[data-mode="' + mode + '"]');
    if (btn) btn.classList.add("active");

    await execCommand("echo '" + mode + "' > " + MODE_FILE);
    var map = {
        auto: "智能模式",
        performance: "游戏模式",
        powersave: "省电模式",
        balance: "均衡模式"
    };
    showToast("切换至: " + (map[mode] || mode), "success");
    updateStatus();
    refreshLog();
}

// ============================================================
// 游戏名单管理
// ============================================================
async function loadGames() {
    try {
        var res = await execCommand("cat " + GAME_LIST + " 2>/dev/null");
        var lines = res.stdout.split("\n").filter(function(s) {
            var clean = s.trim();
            if (!clean) return false;
            if (clean.startsWith("#")) return false;
            return true;
        });
        var list = lines.map(function(s) {
            var idx = s.indexOf("#");
            return idx > 0 ? s.substring(0, idx).trim() : s.trim();
        }).filter(function(s) { return s.length > 0; });

        var container = document.getElementById("gameList");
        if (!list.length) {
            container.innerHTML = '<div class="empty-state">暂无游戏，添加后自动切换游戏模式</div>';
            return;
        }
        container.innerHTML = list.map(function(pkg) {
            return '<div class="game-item">' +
                '<span class="pkg">' + pkg + '</span>' +
                '<button class="del" onclick="removeGame(\'' + pkg + '\')">✕</button>' +
                '</div>';
        }).join("");
    } catch (e) {
        document.getElementById("gameList").innerHTML = '<div class="empty-state">读取失败</div>';
    }
}

async function addGame() {
    var input = document.getElementById("gameInput");
    var pkg = input.value.trim();
    if (!pkg) { showToast("请输入包名", "warning"); return; }
    if (!pkg.includes(".")) { showToast("包名格式错误", "warning"); return; }
    var existing = await execCommand("grep -x \"" + pkg + "\" " + GAME_LIST + " 2>/dev/null");
    if (existing.stdout.trim()) { showToast("该游戏已存在", "warning"); return; }
    await execCommand("echo \"" + pkg + "\" >> " + GAME_LIST);
    input.value = "";
    showToast("已添加: " + pkg, "success");
    loadGames();
    refreshLog();
}

async function removeGame(pkg) {
    await execCommand("grep -v \"^" + pkg + "$\" " + GAME_LIST + " > " + GAME_LIST + ".tmp && mv " + GAME_LIST + ".tmp " + GAME_LIST);
    showToast("已移除: " + pkg, "info");
    loadGames();
    refreshLog();
}

// ============================================================
// 实时状态
// ============================================================
async function updateStatus() {
    try {
        var modeRes = await execCommand("cat " + MODE_FILE + " 2>/dev/null");
        var modeMap = { auto: "智能模式", performance: "游戏模式", powersave: "省电模式", balance: "均衡模式" };
        var mode = modeMap[modeRes.stdout.trim()] || "智能模式";
        document.getElementById("currentMode").textContent = mode;

        var freqRes = await execCommand("cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq 2>/dev/null");
        var freq = parseInt(freqRes.stdout);
        document.getElementById("cpuFreq").textContent = freq ? (freq/1000).toFixed(0) + " MHz" : "--";

        // Prefer the real SoC sensor (type=soc_max) instead of thermal_zone0,
        // which is a charger sensor on the PLG110.
        var tempRes = await execCommand("for z in /sys/class/thermal/thermal_zone*/type; do [ -f \\\"$z\\\" ] || continue; if [ \\\"$(cat \\\"$z\\\")\\\" = soc_max ]; then cat \\\"${z%/type}/temp\\\"; break; fi; done");
        var temp = parseInt(tempRes.stdout);
        document.getElementById("cpuTemp").textContent = temp ? (temp/1000).toFixed(1) + "°C" : "--";

        var gpuUtilRes = await execCommand("v=$(cat /sys/kernel/ged/hal/gpu_utilization 2>/dev/null); [ -n \\\"$v\\\" ] || v=$(cat /sys/kernel/ged/hal/gpu_sum_loading 2>/dev/null); echo \\\"$v\\\"");
        var gpuUtil = parseInt(gpuUtilRes.stdout);
        document.getElementById("gpuUtil").textContent = Number.isFinite(gpuUtil) && gpuUtil >= 0 && gpuUtil <= 100 ? gpuUtil + "%" : "--";
        var gpuNodeRes = await execCommand("if [ -r /sys/kernel/ged/hal/gpu_utilization ]; then echo 'GED 可读'; else echo '未检测'; fi; if [ -w /sys/kernel/ged/hal/custom_upbound_gpu_freq ]; then echo '可写'; fi");
        document.getElementById("gpuStatus").textContent = gpuNodeRes.stdout.trim().replace(/\\n/g, " / ") || "--";

        var battRes = await execCommand("dumpsys battery 2>/dev/null | grep level | awk '{print $2}'");
        document.getElementById("batteryLevel").textContent = battRes.stdout.trim() || "--%";

        var screenRes = await execCommand("dumpsys display 2>/dev/null | grep -i mScreenState | head -1 | grep -oE 'ON|OFF'");
        var screen = screenRes.stdout.trim() || "ON";
        document.getElementById("screenState").textContent = screen === "ON" ? "已唤醒" : "已休眠";

        var gameRes = await execCommand("dumpsys window 2>/dev/null | grep -E 'mCurrentFocus|mFocusedApp' | head -1 | grep -oE '[a-zA-Z][a-zA-Z0-9._]*\\.[a-zA-Z][a-zA-Z0-9._]*'");
        var currentPkg = gameRes.stdout.trim();
        var gameListRes = await execCommand("cat " + GAME_LIST + " 2>/dev/null");
        var games = gameListRes.stdout.split("\n").filter(function(s) { return s.trim().length > 0 && !s.startsWith("#"); });
        var isGame = false;
        for (var i = 0; i < games.length; i++) {
            var pkg = games[i].trim();
            var idx = pkg.indexOf("#");
            if (idx > 0) pkg = pkg.substring(0, idx).trim();
            if (currentPkg === pkg) { isGame = true; break; }
        }
        document.getElementById("gameDetect").textContent = isGame ? "游戏中" : "待机";

        var rustRes = await execCommand("pgrep -f 'yamitsuki_rs' 2>/dev/null && echo '运行中' || echo '未运行'");
        document.getElementById("engineStatus").textContent = "🦀 Rust: " + rustRes.stdout.trim();

        var govRes = await execCommand("cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null");
        var gov = govRes.stdout.trim() || "未知";
        var extraText = "⚙️ 调度器: " + gov;
        if (mode === "游戏模式") {
            extraText += " | 性能优先";
        } else if (mode === "省电模式") {
            extraText += " | 节能优先";
        } else if (mode === "均衡模式") {
            extraText += " | 平衡";
        }
        document.getElementById("extraStatus").textContent = extraText;
    } catch (e) {}
}

// ============================================================
// 日志刷新
// ============================================================
async function refreshLog() {
    try {
        var res = await execCommand("tail -n 30 " + LOG_FILE + " 2>/dev/null");
        document.getElementById("logBox").textContent = res.stdout || "暂无日志";
    } catch (e) {
        document.getElementById("logBox").textContent = "读取失败";
    }
}

// ============================================================
// 功能状态管理
// ============================================================
function saveFeatureState(feature, state) {
    var key = FEATURE_KEYS[feature];
    if (key) {
        try {
            localStorage.setItem(key, JSON.stringify(state));
        } catch(e) {}
    }
}

function getFeatureState(feature) {
    var key = FEATURE_KEYS[feature];
    if (key) {
        try {
            var data = localStorage.getItem(key);
            if (data) return JSON.parse(data);
        } catch(e) {}
    }
    return null;
}

function restoreAllStates() {
    var features = Object.keys(FEATURE_KEYS);
    for (var i = 0; i < features.length; i++) {
        var feature = features[i];
        var state = getFeatureState(feature);
        if (state !== null) {
            applyFeatureUI(feature, state);
        } else {
            applyFeatureUI(feature, 'closed');
        }
    }
}

function applyFeatureUI(feature, state) {
    var btn = document.querySelector('[data-feature="' + feature + '"]');
    var statusEl = document.getElementById(feature + '-status');
    if (!btn || !statusEl) return;

    if (state === 'opened' || state === 'enabled' || state === 'running') {
        statusEl.textContent = '已开启';
        statusEl.className = 'status-tag running';
        btn.textContent = '关闭';
        btn.className = 'btn feature-btn running';
    } else {
        statusEl.textContent = '已关闭';
        statusEl.className = 'status-tag';
        btn.textContent = '开启';
        btn.className = 'btn feature-btn';
    }
}

// ============================================================
// 附加功能控制
// ============================================================
async function toggleFeature(feature) {
    var btn = event.target;
    var statusEl = document.getElementById(feature + '-status');
    var cmdMap = FEATURE_CMDS[feature];
    if (!cmdMap) return;

    var currentState = getFeatureState(feature);
    var isOpen = (currentState === 'opened' || currentState === 'enabled' || currentState === 'running');

    var targetState, cmd;
    if (isOpen) {
        targetState = 'closed';
        cmd = cmdMap.disable;
    } else {
        targetState = 'opened';
        cmd = cmdMap.enable;
    }

    await execCommand('echo "' + cmd + '" > ' + CMD_FILE);
    saveFeatureState(feature, targetState);
    applyFeatureUI(feature, targetState);

    var msg = isOpen ? '已关闭 ' + feature : '已开启 ' + feature;
    showToast(msg, 'success');
}

// ============================================================
// ★★★ 调速器控制 ★★★
// ============================================================
var governorLoaded = false;

window.loadGovernors = function() {
    var container = document.getElementById('governor-list');
    if (!container) return;
    container.innerHTML = '加载中...';

    execCommand('echo "governor:info" > ' + CMD_FILE)
        .then(function() {
            setTimeout(function() {
                execCommand('cat /data/local/tmp/governor_info 2>/dev/null')
                    .then(function(res) {
                        var content = res.stdout || '';
                        var lines = content.split('\n');
                        var current = 'schedutil';
                        var available = [];
                        for (var i = 0; i < lines.length; i++) {
                            var line = lines[i].trim();
                            if (line.startsWith('current:')) {
                                current = line.replace('current:', '').trim();
                            } else if (line.startsWith('available:')) {
                                available = line.replace('available:', '').trim().split(',');
                            }
                        }

                        if (available.length === 0) {
                            container.innerHTML = '<div style="color:var(--text-muted);font-size:12px;">未检测到调速器</div>';
                            return;
                        }

                        container.innerHTML = available.map(function(gov) {
                            var active = gov === current ? ' active' : '';
                            var label = gov.toUpperCase();
                            var desc = '';
                            if (gov === 'performance') desc = '🚀 最高性能';
                            else if (gov === 'powersave') desc = '🌙 极致省电';
                            else if (gov === 'schedutil') desc = '⚖️ 智能平衡';
                            else if (gov === 'ondemand') desc = '📈 按需调频';
                            else if (gov === 'conservative') desc = '📉 平滑调频';
                            else desc = '🔧 自定义';
                            return '<button class="gov-btn' + active + '" data-governor="' + gov + '" onclick="switchGovernor(\'' + gov + '\')">' +
                                '<span class="gov-name">' + label + '</span>' +
                                '<span class="gov-desc">' + desc + '</span>' +
                                '</button>';
                        }).join('');

                        document.getElementById('governor-current-tag').textContent = current.toUpperCase();
                        document.getElementById('governor-status-text').textContent = '当前: ' + current;
                        governorLoaded = true;
                    })
                    .catch(function(e) {
                        container.innerHTML = '<div style="color:var(--text-muted);font-size:12px;">读取失败: ' + e + '</div>';
                    });
            }, 300);
        })
        .catch(function(e) {
            container.innerHTML = '<div style="color:var(--text-muted);font-size:12px;">请求失败: ' + e + '</div>';
        });
};

window.switchGovernor = function(governor) {
    showToast('切换调速器至: ' + governor.toUpperCase(), 'info');
    execCommand('echo "governor:set:' + governor + '" > ' + CMD_FILE)
        .then(function() {
            setTimeout(function() {
                loadGovernors();
                updateStatus();
            }, 500);
        })
        .catch(function(e) {
            showToast('切换失败: ' + e, 'error');
        });
};

// ============================================================
// ★★★ 执行区功能 ★★★
// ============================================================
var spoofPollingInterval = null;

window.runDeviceSpoof = function() {
    var btn = document.getElementById('device-spoof-btn');
    var statusEl = document.getElementById('device-spoof-status');
    var outputEl = document.getElementById('device-spoof-output');

    if (btn.textContent === '执行中...') return;

    outputEl.textContent = '正在执行，请稍候...';
    btn.textContent = '执行中...';
    btn.disabled = true;
    statusEl.textContent = '执行中';
    statusEl.style.color = '#FFD54F';

    execCommand('echo "device_spoof:run" > ' + CMD_FILE)
        .then(function() {
            if (spoofPollingInterval) clearInterval(spoofPollingInterval);
            var resultFile = '/data/adb/modules/yamitsuki_oplus/device_spoof_result.txt';
            var retries = 0;
            var maxRetries = 60;

            spoofPollingInterval = setInterval(function() {
                execCommand('cat ' + resultFile + ' 2>/dev/null')
                    .then(function(res) {
                        var content = res.stdout || '';
                        if (content.includes('[执行完成]')) {
                            var cleanContent = content.replace(/\[执行完成\]/g, '').trim();
                            outputEl.textContent = cleanContent || '执行完成（无输出）';
                            btn.textContent = '执行';
                            btn.disabled = false;
                            statusEl.textContent = '已完成';
                            statusEl.style.color = '#22c55e';
                            clearInterval(spoofPollingInterval);
                            spoofPollingInterval = null;
                        } else if (content.trim()) {
                            outputEl.textContent = content;
                        }
                        retries++;
                        if (retries > maxRetries) {
                            outputEl.textContent = '执行超时，请检查日志';
                            btn.textContent = '执行';
                            btn.disabled = false;
                            statusEl.textContent = '超时';
                            statusEl.style.color = '#ef4444';
                            clearInterval(spoofPollingInterval);
                            spoofPollingInterval = null;
                        }
                    });
            }, 1000);
        })
        .catch(function(e) {
            outputEl.textContent = '发送命令失败: ' + e;
            btn.textContent = '执行';
            btn.disabled = false;
            statusEl.textContent = '失败';
            statusEl.style.color = '#ef4444';
        });
};

// ============================================================
// ★★★ 背景图处理 ★★★
// ============================================================
function applyBackground(dataUrl) {
    if (dataUrl) {
        document.body.style.backgroundImage = 'url(' + dataUrl + ')';
        document.body.style.backgroundSize = 'cover';
        document.body.style.backgroundPosition = 'center';
        document.body.style.backgroundAttachment = 'fixed';
        document.body.style.backgroundColor = '';
        document.body.classList.add('has-bg');
    } else {
        document.body.style.backgroundImage = '';
        document.body.style.backgroundSize = '';
        document.body.style.backgroundPosition = '';
        document.body.style.backgroundAttachment = '';
        document.body.style.backgroundColor = '';
        document.body.classList.remove('has-bg');
    }
}

function pickBg() {
    var input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = function() {
        var file = input.files && input.files[0];
        if (!file) return;
        var reader = new FileReader();
        reader.onload = function(e) {
            var dataUrl = e.target.result;
            applyBackground(dataUrl);
            try {
                localStorage.setItem('yamitsuki-bg', dataUrl);
            } catch(_) {}
            var hint = document.getElementById('bg-hint');
            if (hint) hint.textContent = '已设置自定义背景';
        };
        reader.readAsDataURL(file);
        input.remove();
    };
    input.click();
}

function clearBg() {
    applyBackground(null);
    try {
        localStorage.removeItem('yamitsuki-bg');
    } catch(_) {}
    var hint = document.getElementById('bg-hint');
    if (hint) hint.textContent = '选择一张图片作为背景';
}

function restoreBg() {
    try {
        var savedBg = localStorage.getItem('yamitsuki-bg');
        if (savedBg) {
            applyBackground(savedBg);
            var hint = document.getElementById('bg-hint');
            if (hint) hint.textContent = '已恢复自定义背景';
        }
    } catch(_) {}
}

function onOpacity(el) {
    var val = parseInt(el.value) || 0;
    var display = document.getElementById('v-opacity');
    if (display) display.textContent = val;
    document.documentElement.style.setProperty('--blur', val + 'px');
    try {
        localStorage.setItem('yamitsuki-blur', val.toString());
    } catch(_) {}
}

function restoreBlur() {
    try {
        var savedBlur = localStorage.getItem('yamitsuki-blur');
        if (savedBlur) {
            var val = Math.max(0, Math.min(40, parseInt(savedBlur)));
            document.documentElement.style.setProperty('--blur', val + 'px');
            var rOpacity = document.getElementById('r-opacity');
            var vOpacity = document.getElementById('v-opacity');
            if (rOpacity) rOpacity.value = val;
            if (vOpacity) vOpacity.textContent = val;
        }
    } catch(_) {}
}

// ============================================================
// DOM 加载完成
// ============================================================
document.addEventListener('DOMContentLoaded', function() {
    restoreBg();
    restoreBlur();

    var uninstallBtn = document.getElementById('uninstall-module-btn');
    if (uninstallBtn && !uninstallBtn._listener) {
        uninstallBtn._listener = true;
        uninstallBtn.addEventListener('click', function() {
            if (confirm('⚠️ 确定要卸载 YamiTsuki 模块吗？\n\n此操作不可逆，所有配置和日志将被清除。')) {
                execCommand('sh /data/adb/modules/yamitsuki_oplus/uninstall.sh');
                alert('✅ 卸载脚本已执行。请重启设备或手动在管理器应用中移除模块。');
            }
        });
    }

    var joinBtn = document.getElementById('join-group-btn');
    if (joinBtn && !joinBtn._listener) {
        joinBtn._listener = true;
        joinBtn.addEventListener('click', function() {
            var url = 'https://qm.qq.com/q/81PdbfU7f2';
            execCommand('am start -a android.intent.action.VIEW -d "' + url + '"');
            showToast('正在跳转至QQ群...', 'info');
        });
    }

    restoreAllStates();

    var featureBtns = document.querySelectorAll('.feature-btn');
    featureBtns.forEach(function(btn) {
        if (!btn._listener) {
            btn._listener = true;
            btn.addEventListener('click', function(e) {
                var feature = btn.getAttribute('data-feature');
                if (feature) {
                    event = e;
                    toggleFeature(feature);
                }
            });
        }
    });

    var spoofBtn = document.getElementById('device-spoof-btn');
    if (spoofBtn && !spoofBtn._listener) {
        spoofBtn._listener = true;
        spoofBtn.removeAttribute('onclick');
        spoofBtn.addEventListener('click', window.runDeviceSpoof);
    }

    setTimeout(function() {
        loadGovernors();
    }, 1000);
});

// ============================================================
// 初始化
// ============================================================
async function initMode() {
    var res = await execCommand("cat " + MODE_FILE + " 2>/dev/null");
    var mode = res.stdout.trim() || "auto";
    document.querySelectorAll(".mode-btn").forEach(function(b) {
        b.classList.toggle("active", b.dataset.mode === mode);
    });
}

loadGames();
initMode();
updateStatus();
refreshLog();
setInterval(updateStatus, 2000);
setInterval(refreshLog, 5000);

window.addEventListener("load", function() {
    setTimeout(function() {
        showToast("YamiTsuki V2.0 已启动", "info");
    }, 500);
});