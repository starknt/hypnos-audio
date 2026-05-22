# Hypnos Audio

一款轻量级 Windows 后台工具，可在蓝牙耳机断开时自动静音系统音频，并在重新连接时恢复之前的音量状态。

## 功能特性

### 自动耳机检测
- 通过 Windows MMDevice API 监听音频端点变化
- 根据设备形态（`Headphones` 或 `Headset`）识别耳机/耳麦设备
- 区分设备连接（`ACTIVE`）与断开（`NOTPRESENT` / `UNPLUGGED`）事件

### 智能静音与恢复
- **断开时**：立即保存当前音频状态（静音状态 + 主音量），然后在 500 ms 防抖后静音系统
- **连接时**：恢复之前保存的耳机音量和静音状态
- 使用事件世代计数器防止快速插拔时旧的防抖任务覆盖重连动作

### 通知提示
- 耳机连接/断开时显示 Windows 原生通知（Toast）
- 通知附带应用图标（`appLogoOverride`），一目了然
- 相同标签的通知会替换旧通知，避免在操作中心堆叠

### 系统托盘
- 静默运行在系统托盘，不打扰日常使用
- 托盘菜单选项：
  - **检查更新**：手动触发更新检查
  - **开机启动**：切换 Windows 开机启动注册表项
  - **退出**：优雅关闭应用

### 自动更新
- 集成 Velopack，支持静默自动更新
- 启动时自动检查 GitHub 新版本
- 后台下载安装更新后自动重启

### 单实例运行
- 使用命名互斥锁确保同时只运行一个实例
- 重复启动会自动退出

## 技术架构

| 模块 | 职责 |
|------|------|
| `main.rs` | 入口点、单实例守卫、tokio 运行时、模块组装 |
| `bluetooth.rs` | 音频端点通知客户端、带防抖逻辑的事件循环 |
| `audio.rs` | Windows Core Audio API 封装（静音、音量读写） |
| `state.rs` | 内存中的音频状态快照存储 |
| `notifications.rs` | Windows 通知构建器，支持图标 |
| `tray.rs` | 基于 `tray-icon` + `winit` 的系统托盘图标与菜单 |
| `startup.rs` | Windows 注册表读写，实现开机启动开关 |
| `updater.rs` | Velopack 更新检查 / 下载 / 安装 |

## 构建

需要 Rust 1.85+ 与 Windows 环境。

```bash
cargo build --release
```

Release 二进制文件经过体积优化（约 2.2 MiB），启用了 LTO、单 codegen unit 与 `panic = abort`。

## 配置

设置环境变量 `HYPNOS_GITHUB_REPO`（例如 `owner/repo`）以启用自动更新检查。未设置时将跳过更新检查。

## 最近更新

- **修复**：快速插拔耳机时不再重复静音或发送重复通知
- **优化**：Release 二进制体积从 3.0 MiB 缩减至 2.2 MiB
- **新增**：通知支持显示应用图标

## 许可证

MIT
