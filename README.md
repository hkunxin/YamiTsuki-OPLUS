# YamiTsuki-OPLUS V2.0 — 月之暗面

**欧加真专属底层调度模块** · Rust 引擎

## 项目结构

```
YamiTsuki-OPLUS-V2.0/
├── Cargo.toml              # Rust 项目配置
├── build.rs                # 构建脚本
├── src/
│   ├── main.rs             # 入口 · 守护进程循环 · 命令管道
│   ├── cpu.rs              # CPU 频率 / governor 管理
│   ├── gpu.rs              # GPU 频率控制
│   ├── mode.rs             # 智能模式决策引擎
│   ├── features.rs         # 附加功能（充电/horae/MT隐藏/属性伪装/USB）
│   ├── device_spoof.rs     # 设备特征随机修改
│   └── logger.rs           # 日志系统
├── webroot/
│   ├── index.html          # WebUI 主界面
│   ├── style.css           # 样式
│   └── script.js           # 交互逻辑
├── bin/
│   └── yamitsuki_rs        # 预编译二进制 (aarch64)
├── customize.sh            # KernelSU 安装脚本
├── service.sh              # 启动服务
├── module.prop             # 模块属性
└── game_list.txt           # 默认游戏名单
```

## 编译

```bash
# 本地编译（需 NDK）
cargo build --release --target aarch64-linux-android

# 或使用 GitHub Actions（推送到 main 自动编译）
```

## 功能对照

| 功能 | 状态 |
|------|:----:|
| CPU 大小核识别 + 频率限制 | DONE |
| Governor 切换 (schedutil/performance/powersave/conservative) | DONE |
| GPU 频率控制 | DONE |
| 智能模式（熄屏/低电/充电/游戏检测） | DONE |
| 命令管道协议 (yamitsuki_cmd) | DONE |
| 充电增强 | DONE |
| 禁用 Horae | DONE |
| 禁用硬件叠加层 | DONE |
| 禁用阶梯充电 | DONE |
| 隐藏 MT 管理器 | DONE |
| 属性伪装 | DONE |
| 禁用 USB 调试 | DONE |
| 设备特征随机修改 | DONE |
| 日志系统 | DONE |
