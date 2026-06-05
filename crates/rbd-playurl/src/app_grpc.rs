//! APP gRPC playurl (M4 启用).
//!
//! v1.0: gRPC 未实现, 回退到 WEB API (可提供 8K 通过 fnval=4048).

use anyhow::Result;
use rbd_core::{AudioTrack, VideoTrack};

use crate::client::PlayUrlClient;

/// APP gRPC playurl — v1.0 回退到 WEB API.
///
/// 后续版本将实现真正的 gRPC 客户端 (需要 protobuf 定义和 tonic).
/// 当前通过 WEB 端点 + fnval=4048 获取 DASH/HDR/4K/8K/杜比/AV1.
pub async fn fetch_app_grpc(
    client: &PlayUrlClient,
    bvid: &str,
    cid: u64,
    qn: u32,
) -> Result<(Vec<VideoTrack>, Vec<AudioTrack>)> {
    // 回退到 WEB API (fnval=4048 已覆盖 8K/Dolby/HDR)
    let value = client.api().get_playurl(bvid, cid, qn).await?;
    let (videos, audios, _) = client.parse_tracks(&value)?;
    Ok((videos, audios))
}
