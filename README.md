# RustBiliDownloader (rbd)

> 纯 Rust 的 B 站 (bilibili) 视频下载器 — 对标 BBDown + Yutto, 协程并发, 单一二进制跨平台.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/Charleshzh/RustBiliDownloader/actions/workflows/ci.yml/badge.svg)](https://github.com/Charleshzh/RustBiliDownloader/actions/workflows/ci.yml)

## 项目状态

**v1.0.9** — 全功能可用. 223+ 单元测试, CI (check/test/clippy/fmt) 全绿, 三平台 (Windows/Linux/macOS) 自动发布.

完整设计见 [`设计方案.md`](设计方案.md), 调研背景见 [`调研报告.md`](调研报告.md).

## 功能

| 功能 | 状态 | 说明 |
|------|------|------|
| WEB 扫码登录 + TV 登录 | ✅ | 多 profile 支持, keyring 持久化 |
| 8K / HDR / Dolby Vision / AV1 | ✅ | fnval=4048 WEB API, DASH 音视频分离 |
| UGC 视频下载 | ✅ | 单视频 / 分P |
| 番剧 (Bangumi) | ✅ | ss / ep / md 三种 ID |
| 国际版番剧 (Intl Bangumi) | ✅ | |
| 课程 (Cheese) | ✅ | |
| 收藏夹批量 | ✅ | |
| 合集 (Series) | ✅ | |
| 媒体列表 (MediaList) | ✅ | |
| 内容合集 (Collection) | ✅ | |
| 弹幕下载 (XML → ASS) | ✅ | rayon 并行渲染, 6 种显示模式 |
| 外挂字幕 | ✅ | 5 套 API fallback, JSON/SRT/ASS 互转 |
| DASH 音视频并发下载 + 混流 | ✅ | 协程并发, ffmpeg copy 混流 |
| aria2c 辅助下载 | ✅ | |
| 交互式 track 选择 | ✅ | dialoguer |
| 批量下载 | ✅ | `rbd batch urls.txt` |
| Shell 补全 | ✅ | bash / zsh / fish / powershell / elvish |
| UP 主空间 (Space) | ⚠️ | B站 风控限制, 需改进反爬 |
| TUI | ❌ v1.1 | ratatui |
| HTTP Server | ❌ v1.1 | axum |
| 断点续传 | ❌ v1.1 | |

## 快速开始

### 安装

```bash
# 方式 1: 从源码编译 (需 Rust 1.85+)
git clone https://github.com/Charleshzh/RustBiliDownloader.git
cd RustBiliDownloader
cargo build --release
# 二进制: ./target/release/rbd (或 rbd.exe)

# 方式 2: cargo install
cargo install --git https://github.com/Charleshzh/RustBiliDownloader.git

# 方式 3: GitHub Releases (预编译二进制)
# https://github.com/Charleshzh/RustBiliDownloader/releases
# 产物命名: rbd-v{ver}-{target}.{tar.gz|zip}
```

### 使用

```bash
# === 登录 (获取高画质) ===
rbd login                     # WEB 扫码登录
rbd auth-status               # 查看登录状态
rbd logout                    # 退出登录

# === 下载单个视频 ===
rbd download https://www.bilibili.com/video/BV1GJ411x7h7

# 下载指定分P
rbd download "https://www.bilibili.com/video/BV1xx411c7mD?p=2"

# 仅下载视频 / 仅下载音频
rbd download --video-only https://www.bilibili.com/video/BV1GJ411x7h7
rbd download --audio-only https://www.bilibili.com/video/BV1GJ411x7h7

# 交互式选择画质和编码
rbd download --interactive https://www.bilibili.com/video/BV1GJ411x7h7

# 指定编码优先级
rbd download --vcodec-priority hevc,avc https://www.bilibili.com/video/BV1GJ411x7h7

# === 下载番剧 / 课程 ===
rbd download https://www.bilibili.com/bangumi/play/ss39443
rbd download https://www.bilibili.com/cheese/play/ss5912

# === 查看视频信息 (不下载) ===
rbd info https://www.bilibili.com/video/BV1GJ411x7h7

# === 批量下载 (每行一个 URL, # 开头为注释) ===
rbd batch urls.txt

# === Shell 补全 ===
rbd completions bash > ~/.local/share/bash-completion/completions/rbd
rbd completions powershell > $PROFILE

# === 其他 ===
rbd version                   # 版本信息
rbd --help                    # 查看完整帮助
rbd download --help           # 查看 download 子命令帮助
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

## 架构

### Crate 列表

| Crate | 职责 | 测试 |
|---|---|---|
| `rbd-foundation` | 错误 / 日志 / 路径 / 进度 / 配置 / 编码 / 重试 / 限流 / 模板 | 18 |
| `rbd-core` | BilibiliId (9 种) / BV↔AV / WBI / URL 解析 / 9 Extractor / API 客户端 | 43 |
| `rbd-auth` | 多 profile 登录 + WEB QR + TV QR + keyring + cookie | 37 |
| `rbd-playurl` | WEB / HTML5 / TV / INTL / APP 5-mode playurl | 6 |
| `rbd-danmaku` | XML / JSON / Protobuf 解析 → ASS 渲染 (rayon 并行) | 50 |
| `rbd-subtitle` | 5 API fallback + JSON/SRT/ASS 互转 | 31 |
| `rbd-downloader` | Range + 协程并发 + aria2c + Job 状态机 (cancel/pause/resume) | 32 |
| `rbd-muxer` | DASH remux (ffmpeg copy) + 策略选择 | 6 |
| `rbd-cli` | clap CLI + dialoguer + batch + completions | binary |

### 下载管线

```
URL → parse_url() → NormalizedId
  → ExtractorRegistry → Extractor::extract()
  → PlayUrlClient::fetch() [Web→TV→Html5 fallback × quality tiers]
  → DownloadManager::download_tracks_concurrent()
  → Muxer::mux() [ffmpeg copy]
  → .mp4
```

## CI

| Job | 命令 |
|-----|------|
| check | `cargo check --workspace` |
| test | `cargo test --workspace --lib` |
| clippy | `cargo clippy --workspace -- -D warnings` |
| fmt | `cargo fmt --all -- --check` |

Release: tag `v*` 触发三平台构建 → GitHub Release (产物: `rbd-v{ver}-{target}.{tar.gz|zip}`).

## 已知限制

| 限制 | 说明 |
|------|------|
| UP 主空间 (-403) | B站 风控要求更强的认证, BBDown 同样不支持 (抛异常) |
| APP gRPC | 刻意不实现 — fnval=4048 WEB API 已覆盖 8K/HDR/Dolby/AV1 |
| 纯 Rust m4s→mp4 | 当前委托 ffmpeg copy, v1.1 计划 |
| 国际版番剧 | 仅 URL 解析, 未做端到端验证 |

## 路线图

- **v1.0** — 完整功能对标 BBDown / Yutto ✅
- **v1.1** — TUI + HTTP server + 断点续传 + 纯 Rust m4s→mp4
- **v2.0** — GUI (Tauri 2.0)

## License

MIT — 见 [LICENSE](LICENSE).
