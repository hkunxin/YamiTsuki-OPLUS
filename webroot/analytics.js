function parseAnalyticsCsv(line) {
    return line.split(',').map(function(value) { return value.trim(); });
}

function drawPowerChart(rows) {
    var canvas = document.getElementById('powerChart');
    if (!canvas) return;
    var width = canvas.clientWidth || 400;
    var height = 220;
    var ratio = window.devicePixelRatio || 1;
    canvas.width = width * ratio;
    canvas.height = height * ratio;
    var ctx = canvas.getContext('2d');
    ctx.scale(ratio, ratio);
    ctx.clearRect(0, 0, width, height);
    if (!rows.length) {
        ctx.fillStyle = '#8b9bb0';
        ctx.font = '12px sans-serif';
        ctx.fillText('暂无历史采样，守护进程将在后台低频记录', 16, height / 2);
        return;
    }
    var left = 34;
    var top = 14;
    var plotWidth = width - left - 12;
    var plotHeight = height - top - 24;
    var maxPower = Math.max(1, Math.ceil(Math.max.apply(null, rows.map(function(row) { return row.power; }))));
    ctx.strokeStyle = 'rgba(255,255,255,.08)';
    ctx.lineWidth = 1;
    for (var i = 0; i <= 4; i++) {
        var y = top + plotHeight * i / 4;
        ctx.beginPath();
        ctx.moveTo(left, y);
        ctx.lineTo(width - 12, y);
        ctx.stroke();
        ctx.fillStyle = '#65758a';
        ctx.font = '10px monospace';
        ctx.fillText((maxPower * (4 - i) / 4).toFixed(1), 3, y + 3);
    }
    function drawLine(key, color, max) {
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        rows.forEach(function(row, index) {
            var x = left + plotWidth * index / Math.max(1, rows.length - 1);
            var y = top + plotHeight * (1 - Math.max(0, Math.min(max, row[key])) / max);
            if (index === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
        });
        ctx.stroke();
    }
    drawLine('power', '#06b6d4', maxPower);
    drawLine('battery', '#22c55e', 100);
}

async function loadAnalytics() {
    var dateRes = await execCommand('date +%Y%m%d');
    var date = dateRes.stdout.trim() || 'current';
    var statusRes = await execCommand('cat /data/local/tmp/yamitsuki_power_status 2>/dev/null');
    var session = 0;
    statusRes.stdout.split(/\r?\n/).forEach(function(line) {
        var pair = line.split('=');
        if (pair.length > 1 && pair[0].trim() === 'session') session = Number(pair[1].trim()) || 0;
    });
    var powerRes = await execCommand('cat ' + MODULE_DIR + '/data/power-' + date + '.csv 2>/dev/null');
    var rows = powerRes.stdout.trim().split(/\r?\n/).slice(1).filter(Boolean).slice(-60).map(function(line) {
        var values = parseAnalyticsCsv(line);
        return { battery: Number(values[1]) || 0, power: Number(values[4]) || 0 };
    });
    drawPowerChart(rows);
    var meta = document.getElementById('analytics-meta');
    if (meta) meta.textContent = rows.length ? '本次放电会话 #' + (session || '-') + ' · 当日采样 ' + rows.length + ' 条 · 充电期间不参与统计，每次充电后重新统计' : '暂无历史采样数据';

    var appRes = await execCommand('cat ' + MODULE_DIR + '/data/apps-' + date + '.csv 2>/dev/null; echo; cat ' + MODULE_DIR + '/data/apps-$(date -d yesterday +%Y%m%d 2>/dev/null).csv 2>/dev/null');
    var apps = {};
    var latestSession = 0;
    var appRows = [];
    appRes.stdout.trim().split(/\r?\n/).filter(Boolean).forEach(function(line) {
        if (line.indexOf('timestamp,') === 0) return;
        var values = parseAnalyticsCsv(line);
        appRows.push(values);
        var s = Number(values[6]) || 0;
        if (s > latestSession) latestSession = s;
    });
    var targetSession = session || latestSession;
    appRows.forEach(function(values) {
        var s = Number(values[6]) || 0;
        if (s !== targetSession) return;
        var pkg = values[1] || '未知应用';
        if (!apps[pkg]) apps[pkg] = { label: values[2] || pkg, power: 0, energy: 0, samples: 0 };
        apps[pkg].power += Number(values[3]) || 0;
        apps[pkg].energy += Number(values[4]) || 0;
        apps[pkg].samples++;
    });
    var sorted = Object.keys(apps).map(function(pkg) { return { pkg: pkg, data: apps[pkg] }; }).sort(function(a, b) { return b.data.energy - a.data.energy; });
    var table = document.getElementById('appPowerTable');
    if (table) table.innerHTML = sorted.length ? '<div class="power-row power-head"><span>应用</span><span>估算功率</span><span>估算电量</span></div>' + sorted.slice(0, 20).map(function(item) { return '<div class="power-row"><span title="' + item.pkg + '">' + item.data.label + '<small>' + item.pkg + '</small></span><b>' + (item.data.power / item.data.samples).toFixed(2) + ' W</b><b>' + item.data.energy.toFixed(2) + ' mAh</b></div>'; }).join('') : '<div class="empty-state">当前放电会话暂无应用耗电记录</div>';
}
