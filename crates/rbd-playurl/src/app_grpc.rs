//! APP gRPC playurl.
//!
//! ## v1.0 决策: 不实现 APP gRPC, 回退到 WEB API
//!
//! **背景**: B 站 APP 端使用 gRPC 协议获取播放地址, 可提供更全的清晰度选项.
//! 社区项目 (BBDown, yutto) 通常实现 APP gRPC 以获取 8K/HDR/杜比/AV1 等格式.
//!
//! **v1.0 决策**: **刻意不实现 APP gRPC**, 原因如下:
//!
//! 1. WEB API 已覆盖全部格式: 使用 `fnval=4048` (bit flags: 8 | 16 | 1024 | 3000)
//!    已解锁 WEB 端可用的所有格式, 包括:
//!    - 8K 超高清 (bit 1024)
//!    - HDR / Dolby Vision (bit 16)
//!    - AV1 编码 (bit 8)
//!    - 杜比全景声 / Hi-Res 无损音频 (bit 3000)
//!
//! 2. APP gRPC 需额外依赖: tonic + protobuf 定义编译, 增加约 3MB 二进制体积
//!    和 20s 编译时间, 但功能层面无增量.
//!
//! 3. 维护成本: APP gRPC 接口更不稳定 (B 站频繁更新 protobuf 定义),
//!    且需要独立的登录方案 (APP token vs WEB cookie).
//!
//! **后续计划 (v1.1+)**: 当以下场景出现时考虑实现:
//! - WEB API 的 fnval 不再提供 8K/Dolby/HDR
//! - 用户反馈需要 APP 端独家格式 (如赛事直播的原画 PRO)
//! - protobuf 定义稳定且有成熟的 Rust protobuf 生态
//!
//! **实现参考** (供后续开发):
//! - Proto: `bilibili.app.playurl.v1.PlayViewReply`
//! - 接口: `grpc://grpc.biliapi.net/bilibili.app.playurl.v1.PlayURL/PlayView`
//! - 社区实现: BBDown/PlayURLProto.cs, yutto/api/ugc_video.py
//! - Rust 依赖: tonic v0.12 + prost v0.13 (已在 workspace Cargo.toml 预留)

use anyhow::Result;
use rbd_core::{AudioTrack, VideoTrack};

use crate::client::PlayUrlClient;

/// APP gRPC playurl — v1.0 回退到 WEB API.
///
/// 这是 v1.0 的刻意决策 (详见模块文档).
/// WEB API (`fnval=4048`) 已覆盖 8K/HDR/Dolby/AV1/杜比全景声,
/// APP gRPC 在功能层面无增量, 但在维护成本和二进制体积方面有额外开销.
///
/// # 实现
///
/// 直接复用 `PlayUrlClient` 的 WEB API 获取和解析逻辑.
/// 调用链: `api.get_playurl()` → `client.parse_tracks()`.
pub async fn fetch_app_grpc(
    client: &PlayUrlClient,
    bvid: &str,
    cid: u64,
    qn: u32,
) -> Result<(Vec<VideoTrack>, Vec<AudioTrack>)> {
    // WEB API fnval=4048 已覆盖 8K/Dolby/HDR
    // 参见模块级文档了解完整决策背景
    let value = client.api().get_playurl(bvid, cid, qn).await?;
    let (videos, audios) = client.parse_tracks(&value)?;
    Ok((videos, audios))
}
