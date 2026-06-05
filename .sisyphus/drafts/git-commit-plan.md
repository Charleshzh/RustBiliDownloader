# 分批提交方案 (Git Commit Plan)

## 策略
按 **Milestone 顺序**分批提交，每批一个独立 commit，确保 bisect 安全。
排除 `tmp/` 测试目录、`Cargo.lock`（后续再提交）、`.gitignore` 中与 tmp 相关条目。

---

## Commit 1: 基础设施与 Workspace 初始化

```text
初始化 Cargo Workspace 与项目基础设施

- 添加顶层 Cargo.toml：定义 9 crate workspace 成员
- 添加 rust-toolchain.toml：固定 Rust 1.88.0（兼容现代 crate 生态）
- 添加 .cargo/config.toml：registry 镜像配置
- 添加 .gitignore：Rust / IDE / 临时文件忽略规则
- 添加 LICENSE：MIT 许可证
- 添加 README.md：项目简介与快速开始
```

**文件**：
- `Cargo.toml`
- `rust-toolchain.toml`
- `.cargo/config.toml`
- `.gitignore`
- `LICENSE`
- `README.md`

---

## Commit 2: 调研与设计方案文档

```text
添加调研报告与架构设计方案

- 调研报告.md：BBDown / yutto / FastestBilibiliDownloader 三项目深度调研
  - BBDown master 分支完整源码分析（C#）
  - yutto Python + Rust biliass 架构分析
  - Fastest Go 项目缺陷分析
  - Rust 改写建议与 crate 选型
- 设计方案.md：RustBiliDownloader v1.0 完整设计
  - 9 crate 模块划分与职责
  - CLI + TUI 交互设计（v1.0 仅 CLI）
  - 内部 trait / struct 接口定义
  - 用户 16 项关键决策记录
```

**文件**：
- `调研报告.md`
- `设计方案.md`

---

## Commit 3: rbd-foundation — 基础工具 crate

```text
实现 rbd-foundation：通用基础工具集（11 模块）

- error.rs: RbdError 17 变体 + anyhow::Result 别名
- log.rs: 单文件 / 双输出 / stderr 三模式日志
- path.rs: sanitize_filename / unique_path / temp_dir
- progress.rs: 单条 / 多条进度抽象
- config.rs: TOML 配置 load/save，支持 dirs 跨平台定位
- ratelimit.rs: governor 限流器封装（new_limiter / tick / tick_n）
- retry.rs: ExponentialBackoff 三档回退 + with_retry! 宏
- template.rs: tera 模板引擎 + 13 变量 + pattern_to_tera 转换
- codec.rs: hex / url_encode / base58 / md5 / sha1
- version.rs: 版本常量
- locale.rs: i18n 占位（v1.0 仅中文）

含单元测试 18 个，覆盖 config/ratelimit/retry/template/codec/path。
```

**文件**：`crates/rbd-foundation/**/*`

---

## Commit 4: rbd-core — 核心协议与解析层

```text
实现 rbd-core：B站核心协议、URL 解析、Extractor 框架

- id.rs: 9 种 NormalizedId（UGC/Bangumi/Cheese/Fav/Space/MediaList/Series/ShortLink/RawId）
  - URL 正则解析器，支持 12 种 URL 形式
  - 13 个单元测试
- bv.rs: BV <-> AV 互转算法（BBDown 兼容）+ 4 测试
- wbi.rs: WBI 签名算法（MIXIN_TABLE 66 项 + md5）+ 2 测试
- model.rs: VInfo / Page / Track / VideoTrack / AudioTrack / SubtitleTrack
- api.rs: BilibiliApi HTTP 客户端
  - reqwest + Arc<Client> 可 Clone
  - WBI 签名缓存与自动刷新
  - Cookie / Header 管理
  - 14 个 WEB API 端点封装（view/pagelist/playurl/subtitle/...）
  - 3 个单元测试
- playurl.rs: DASH / durl 响应解析
  - 5 种 ApiMode（Web/TV/Html5/App/Intl）
  - into_tracks() 提取 video/audio 轨道
  - 含 is_combined 标记（durl 合并流 vs DASH 分离流）
  - 6 个单元测试
- proto.rs: ApiMode 枚举 + 序列化
- extractor.rs: Extractor trait + ExtractorRegistry
  - 8 个 Fetcher（Normal/Bangumi/IntlBangumi/Cheese/FavList/MediaList/Series/Space）
  -  skeleton 已替换为真实实现

含单元测试 36 个。
```

**文件**：`crates/rbd-core/**/*`

---

## Commit 5: rbd-playurl — PlayUrl 多模式客户端

```text
实现 rbd-playurl：多模式播放地址获取与降级链

- client.rs: PlayUrlClient，统一入口
- web.rs: WEB API 模式（fnval=4048，DASH + 4K/HDR/Dolby/AV1）
- web_bangumi.rs: 番剧 WEB API 模式
- tv.rs: TV 端 API 模式
- html5.rs: HTML5 兼容模式（durl 合并流）
- app_grpc.rs: APP gRPC 模式（fallback 到 WEB API）
- fallback.rs: FallbackChain 自动降级（Web → WebBangumi → TV → Html5 → App）

含单元测试 6 个。
```

**文件**：`crates/rbd-playurl/**/*`

---

## Commit 6: rbd-downloader — 并发下载引擎

```text
实现 rbd-downloader：Range 分块并发下载引擎

- range.rs: HTTP Range 请求分块计算（ceil 除法 + 用户指定线程数）
- parallel.rs: 多线程并行下载
  - 流式磁盘写入（Arc<Mutex<File>> + seek+write，避免内存缓冲）
  - Semaphore 并发控制
- aria2c.rs: aria2c RPC 客户端（RPC 调用 + 状态轮询）
- manager.rs: DownloadManager 下载编排
  - 支持 video_only / audio_only / 双轨模式
  - 音视频并发下载（tokio::join!）
- event.rs: 下载事件系统（进度 / 完成 / 错误）
- progress.rs: 下载进度数据模型

含单元测试 7 个。
```

**文件**：`crates/rbd-downloader/**/*`

---

## Commit 7: rbd-muxer — 视频混流与元数据

```text
实现 rbd-muxer：DASH 混流、ffmpeg 封装、元数据注入

- strategy.rs: 混流策略选择（DashCopy / FFmpeg / Skip）
  - 根据 codec / HDR / Dolby Vision 自动选择
- dash_copy.rs: DASH 直接拼接（fallback 到 ffmpeg merge_copy）
- ffmpeg.rs: FFmpeg 命令构建与执行
  - 实际调用 ffmpeg 进程（.status()? + 退出码检查）
  - 支持 copy / re-encode / cover 注入 / metadata
- command.rs: 命令行参数构造
- detector.rs: 媒体格式检测
- metadata.rs: 封面 / 章节 / 元数据注入

含单元测试 6 个。
```

**文件**：`crates/rbd-muxer/**/*`

---

## Commit 8: rbd-auth — 多配置认证与扫码登录

```text
实现 rbd-auth：多 profile 认证、扫码登录、Cookie 管理

- profile.rs: AuthProfileModel（pydantic 风格）+ AuthFileModel
  - 多 profile 增删查改（default / profile1 / profile2）
  - TOML 持久化 + 0o600 权限
- web_qr.rs: WEB 端扫码登录
  - QR 生成 → 2s 轮询 × 90 次（180s 超时）
  - Set-Cookie 头提取 SESSDATA / bili_jct
- tv_qr.rs: TV 端扫码登录（同上流程）
- cookie.rs: Cookie 解析与格式化
- keyring_store.rs: 系统 keyring 存储（save/load/delete/list）
- buvid.rs: buvid3/buvid4/b_nut 生成
- refresh.rs: Cookie 自动刷新

含单元测试 31 个。
```

**文件**：`crates/rbd-auth/**/*`

---

## Commit 9: rbd-subtitle — 字幕获取与格式转换

```text
实现 rbd-subtitle：字幕获取、格式转换、降级链

- model.rs: SubtitleTrack / SubtitleFormat 数据模型
- fetch.rs: B站字幕 API 调用（player/wbi/v2 主接口）
- fallback.rs: 5 级降级链（player.so → view → dm → wbi → 空）
  - 当前实现 #1-#2，#3-#5 留 TODO
- convert.rs: 格式转换（JSON ↔ SRT ↔ ASS）
- format.rs: 字幕格式检测与解析

含单元测试 28 个。
```

**文件**：`crates/rbd-subtitle/**/*`

---

## Commit 10: rbd-danmaku — 弹幕解析与 ASS 渲染

```text
实现 rbd-danmaku：弹幕解析、ASS 渲染、并行处理

- model.rs: Danmaku / DanmakuList / DanmakuType 数据模型
- reader.rs: 多格式读取
  - XML（V1/V2 格式）
  - JSON（Web 端格式）
  - Protobuf（TODO：DmSegMobileReply prost 解析）
- color.rs: B站颜色 → ASS \c&HBBGGRR& 转换，alpha = 255 - (opacity*255)
- layout.rs: 弹幕排布算法（row 冲突检测 + 替代行查找）
- options.rs: BlockOptions / DanmakuOptions 配置
- render.rs: ASS 渲染器
  - 6 种弹幕模式（滚动/顶部/底部/逆向/精确/高级）
  - 旋转/动画/颜色/透明度/字号
  - rayon 并行 render_batch_parallel
- writer.rs: ASS 文件输出（Script Info / V4+ Styles / Events）

含单元测试 48 个。
```

**文件**：`crates/rbd-danmaku/**/*`

---

## Commit 11: rbd-cli — 命令行入口与下载流程编排

```text
实现 rbd-cli：CLI 入口、8 子命令、完整下载流程编排

- main.rs: clap CLI 定义，8 子命令（download/login/logintv/auth/logout/info/version）
- commands/download.rs: 完整下载流程
  - URL → parse_url → ExtractorRegistry → extract → PlayUrlClient → select_best → DownloadManager → mux
  - video_only / audio_only / interactive 模式
  - 进度条实时更新（indicatif）
  - SSRF URL 白名单验证 + 路径穿越防护
- commands/login.rs: login / logintv / auth-status / logout 子命令
- commands/info.rs: URL 解析信息显示
- progress.rs: CliProgress（indicatif 包装）
- config.rs: CLI 配置加载（复用 rbd-foundation::config）

连接全部 8 个内部 crate，端到端下载验证通过。
```

**文件**：`crates/rbd-cli/**/*`

---

## Commit 12: 代码审查修复 — 安全、稳定性与质量

```text
修复代码审查发现的安全与稳定性问题

- download 管线：
  - manager.rs 支持 video_only / audio_only 模式（原强制要求音频导致失败）
  - parallel.rs 流式磁盘写入替代内存缓冲（防 OOM）
  - download.rs 模板变量路径穿越过滤（.. 组件）
  - download.rs SSRF 下载 URL 白名单验证
- 混流：
  - dash_copy.rs 委托 ffmpeg merge_copy（原字节拼接产无效 MP4）
  - ffmpeg.rs 实际执行 .status()? + 退出码检查
- 认证：
  - web_qr.rs / tv_qr.rs 轮询循环 2s×90 次 + Set-Cookie 头提取（原只轮询一次）
- API：
  - api.rs BilibiliApi 实现 Clone（Arc<Client>），减少重复实例化
  - FNVAL_DASH_ALL 提取为命名常量
- 质量：
  - 修复 9 处 clippy 警告
  - RbdError 逐变体标记 allow(dead_code)
  - 6 个 skeleton Extractor 补全真实实现
  - CLI flags 全接入（quality / vcodec / workers / interactive）
  - progress 实时回调连接

验证：cargo test --workspace 187 passed, cargo clippy 零错误。
```

**文件**：跨多个 crate 的修改（约 18 文件）

---

## Commit 13: 端到端验证修复 — durl 合并流与显示优化

```text
修复端到端验证发现的 durl 格式与显示问题

- durl 合并流处理：
  - model.rs VideoTrack 新增 is_combined 字段
  - html5.rs durl 轨道标记 is_combined = true
  - playurl.rs DASH 轨道标记 is_combined = false
  - download.rs 合并流输出 .mp4 扩展名，跳过混流，日志提示"含音视频编码流"
- 显示优化：
  - id.rs NormalizedId 实现 Display trait（原输出 Discriminant(0)）
  - info 命令输出人类可读类型名（如"UGC 视频: BV1GJ411x7h7"）

验证：未登录下载 51MB mp4（h264+aac 双流，ffprobe 通过），
      login QR 生成与轮询正常，info 解析正确。
```

**文件**：`crates/rbd-core/src/model.rs`, `crates/rbd-playurl/src/html5.rs`, `crates/rbd-core/src/playurl.rs`, `crates/rbd-cli/src/commands/download.rs`, `crates/rbd-core/src/id.rs`

---

## 执行顺序

```bash
# 步骤 1-13 按顺序执行
git add <commit-1-files> && git commit -m "<commit-1-message>"
git add <commit-2-files> && git commit -m "<commit-2-message>"
...
git add <commit-13-files> && git commit -m "<commit-13-message>"

# 最后
git tag v1.0.0
```

---

## 审核清单

- [ ] 分批粒度合理？（每批 1 个 milestone / 1 个主题）
- [ ] commit 消息中文且足够详细？
- [ ] 文件分组无遗漏？（对比 git ls-files 输出）
- [ ] 顺序正确？（依赖方向：foundation → core → playurl/downloader/muxer → auth/subtitle → danmaku → cli → fixes）
- [ ] Cargo.lock 是否应在某一批提交？（建议第 1 批或最后 1 批）
- [ ] tmp/ 目录是否排除？（是，已在 .gitignore 中）
