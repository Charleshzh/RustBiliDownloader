# RustBiliDownloader (rbd)

> 纯 Rust 的 B 站 (bilibili) 视频下载器 — 完整复刻 BBDown + Yutto 核心能力, 协程并发更快, 单一二进制跨平台.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/Charleshzh/RustBiliDownloader/actions/workflows/ci.yml/badge.svg)](https://github.com/Charleshzh/RustBiliDownloader/actions/workflows/ci.yml)

## 项目状态

**v1.0** — 全功能可用. 193+ 单元测试, CI (check/test/clippy/fmt) 全绿.

完整设计见 [`设计方案.md`](设计方案.md), 调研背景见 [`调研报告.md`](调研报告.md).

## 功能

| 功能 | 状态 | 说明 |
|------|------|------|
| WEB 扫码登录 + TV 登录 | ✅ | 多 profile 支持, keyring 持久化 |
| 8K / HDR / Dolby Vision / AV1 | ✅ | fnval=4048 WEB API, 自动选择最佳质量 |
| UGC 视频下载 | ✅ | 单视频 / 分P |
| 番剧 (Bangumi) | ✅ | ss / ep / md 三种 ID |
| 国际版番剧 (Intl Bangumi) | ✅ | |
| 课程 (Cheese) | ✅ | |
| 收藏夹批量 | ✅ | |
| 合集 (Series) | ✅ | |
| UP 主空间批量 | ✅ | |
| 媒体列表 | ✅ | |
| 弹幕下载 (XML → ASS) | ✅ | rayon 并行渲染, 6 种模式 |
| 外挂字幕 | ✅ | 5 套 API, JSON/SRT/ASS 互转 |
| DASH 音视频分离下载 + 混流 | ✅ | 音视频协程并发, ffmpeg 混流 |
| aria2c 辅助下载 | ✅ | |
| 交互式 track 选择 | ✅ | dialoguer |
| 批量下载 | ✅ | `rbd batch urls.txt` |
| Shell 补全 | ✅ | bash / zsh / fish / powershell / elvish |
| TUI | ❌ v1.1 | ratatui |
| HTTP Server | ❌ v1.1 | axum |
| 断点续传 | ❌ v1.1 | |

## 快速开始

### 安装

```bash
# 方式 1: cargo install (需 Rust 1.85+)
cargo install --git https://github.com/Charleshzh/RustBiliDownloader.git

# 方式 2: 从源码编译
git clone https://github.com/Charleshzh/RustBiliDownloader.git
cd RustBiliDownloader
cargo build --release

# 方式 3: GitHub Releases (预编译二进制)
# 访问 https://github.com/Charleshzh/RustBiliDownloader/releases 下载对应平台
```

### 使用

```bash
# 下载单个视频
rbd download https://www.bilibili.com/video/BV1GJ411x7h7

# 下载指定分P
rbd download "https://www.bilibili.com/video/BV1xx411c7mD?p=2"

# 仅下载视频 (无音频)
rbd download --video-only https://www.bilibili.com/video/BV1GJ411x7h7

# 交互式选择画质和编码
rbd download --interactive https://www.bilibili.com/video/BV1GJ411x7h7

# 下载番剧
rbd download https://www.bilibili.com/bangumi/play/ss39443

# 登录 (获取高画质)
rbd login

# 查看登录状态
rbd auth-status

# 退出登录
rbd logout

# 查看视频信息 (不下载)
rbd info https://www.bilibili.com/video/BV1GJ411x7h7

# 批量下载 (每行一个 URL, # 开头为注释)
rbd batch urls.txt

# 生成 Shell 补全
rbd completions bash > ~/.local/share/bash-completion/completions/rbd
```

### 命令行参数

```
rbd download [OPTIONS] <URL>

  -o, --output-dir <DIR>          输出目录
  -q, --quality <QN>             最低画质 (127=8K, 125=4K, 116=1080P60, 80=1080P)
  --vcodec-priority <CODECS>     编码优先级 (hevc,avc,av1)
  -n, --num-workers <N>          并发下载线程数
  --video-only                   仅下载视频
  --audio-only                   仅下载音频
  --no-danmaku                   跳过弹幕
  --no-subtitle                  跳过字幕
  --no-cover                     跳过封面
  --interactive                  交互式选择 track
  --aria2c                       使用 aria2c 下载
```

## Crate 列表

| Crate | 职责 | 状态 |
|---|---|---|
| `rbd-foundation` | 错误 / 日志 / 路径 / 进度 / 配置 / 编码 / 重试 / 限流 / 模板 | ✅ |
| `rbd-core` | BilibiliId (9 种) / BV↔AV / WBI / URL 解析 / 9 个 Extractor / API 客户端 | ✅ |
| `rbd-auth` | 多 profile 登录 + WEB QR + TV QR + keyring + cookie | ✅ |
| `rbd-playurl` | WEB / HTML5 / TV / INTL / APP 5-mode playurl | ✅ |
| `rbd-danmaku` | XML / JSON / Protobuf 解析 → ASS 渲染 (rayon 并行) | ✅ |
| `rbd-subtitle` | 5 套 API + JSON/SRT/ASS 互转 | ✅ |
| `rbd-downloader` | Range 下载 + 协程并发 + aria2c + Job 状态机 | ✅ |
| `rbd-muxer` | DASH remux (ffmpeg copy) + 策略选择 | ✅ |
| `rbd-cli` | clap CLI + dialoguer + 批量 + 补全 | ✅ |

## CI

GitHub Actions: [`.github/workflows/ci.yml`](.github/workflows/ci.yml)

- **check**: `cargo check --workspace`
- **test**: `cargo test --workspace`
- **clippy**: `cargo clippy --workspace -- -D warnings`
- **fmt**: `cargo fmt --all -- --check`

## 已知限制

| 限制 | 说明 |
|------|------|
| UP 主空间 (-403) | B站 风控要求更强的认证, 参考 BBDown 同样不支持 (抛异常 "暂不支持该功能") |
| APP gRPC | 刻意不实现 — fnval=4048 WEB API 已覆盖 8K/HDR/Dolby/AV1 全部格式 |
| 合集 / 媒体列表 | URL 解析已完成, 未用真实链接做端到端验证 |
| 纯 Rust m4s→mp4 | 当前委托 ffmpeg, v1.1 计划 |

## 路线图

- **v1.0** — 完整功能对标 BBDown / Yutto ✅
- **v1.1** — TUI + HTTP server + 断点续传 + 纯 Rust m4s→mp4
- **v2.0** — GUI (Tauri 2.0)

## License

MIT — 见 [LICENSE](LICENSE).
