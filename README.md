# MoonTune-OPLUS V2.0 — 月之暗面

**欧加真专属底层调度模块** · Rust 引擎

## 项目结构

```
MoonTune-OPLUS-V2.0/
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
| GPU 识别与 GED 负载读取 | PLG110 适配 |
| GPU 频率上限控制 | 条件支持，取决于节点权限 |
| FAS GPU 负载感知 | 部分实现 |
| 智能模式（熄屏/低电/游戏检测） | 已实现，需真机验证 |
| 命令管道协议 (yamitsuki_cmd) | DONE |
| WebUI SoC 温度与 GPU 状态 | 已适配 PLG110 |
| 充电增强 | 条件支持，不保证节点存在 |
| 禁用 Horae | 条件支持 |
| 禁用硬件叠加层 | 条件支持 |
| 禁用阶梯充电 | 条件支持 |
| 隐藏 MT 管理器 | 条件支持 |
| 属性伪装 | 高风险，需手动验证 |
| 禁用 USB 调试 | 条件支持 |
| 设备特征随机修改 | 条件支持 |
| 日志系统 | DONE |
| IO 调度动态节点扫描 | 已实现 |
| SCX sched_ext | 仅内核支持时可用 |
