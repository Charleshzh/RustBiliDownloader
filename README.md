# RustBiliDownloader (RBD)

> 纯 Rust 的 B 站 (bilibili) 视频下载器 — 完整复刻 BBDown + Yutto 核心能力, 协程并发更快, 单一二进制跨平台.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 1.83+](https://img.shields.io/badge/Rust-1.83%2B-orange.svg)](https://www.rust-lang.org)

## 项目状态

**v0.1.0 开发中** — 当前处于 M0 阶段 (Cargo workspace 初始化).

完整设计见 [`设计方案.md`](设计方案.md), 调研背景见 [`调研报告.md`](调研报告.md).

## Crate 列表

| Crate | 职责 | v1.0 状态 |
|---|---|---|
| `rbd-foundation` | 错误 / 日志 / 路径 / 进度 / 配置 | ✅ |
| `rbd-core` | BilibiliId / WBI / BV<->AV / 8 个 Extractor | ✅ |
| `rbd-auth` | 多 profile 登录 + keyring | ✅ |
| `rbd-playurl` | 4-mode WEB + APP gRPC (8K 备选) | ✅ |
| `rbd-danmaku` | 自研 ASS 渲染 (rayon 并行) | ✅ |
| `rbd-subtitle` | 5 套字幕 API + 格式互转 | ✅ |
| `rbd-downloader` | Range + 协程并发 + aria2c | ✅ |
| `rbd-muxer` | DASH copy + ffmpeg-sidecar | ✅ |
| `rbd-cli` | clap + dialoguer 入口 | ✅ |
| `rbd-tui` | ratatui 终端 UI | ❌ v1.1 |
| `rbd-server` | axum HTTP server | ❌ v1.1 |

## 快速开始 (开发版)

```bash
# 编译
cargo build --release

# 运行 (单条视频)
./target/release/rbd https://www.bilibili.com/video/BVxxxxxx

# 登录
./target/release/rbd login

# 查看帮助
./target/release/rbd --help
```

## 路线图

- **v1.0** (M0-M11, 12 周) — 完整功能对标 BBDown
- **v1.1** — TUI + HTTP server + 断点续传
- **v2.0** — GUI (Tauri 2.0) + 多语言

## License

MIT — 见 [LICENSE](LICENSE).
