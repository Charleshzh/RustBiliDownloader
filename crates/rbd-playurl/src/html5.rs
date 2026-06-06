//! HTML5 playurl 端点.

use anyhow::Result;
use rbd_core::{AudioTrack, VideoTrack};

use crate::client::PlayUrlClient;

/// HTML5 playurl 端点 — 最低优先级 fallback, 仅 480P, 免登录可看.
pub async fn fetch_html5(
    client: &PlayUrlClient,
    bvid: &str,
    cid: u64,
    qn: u32,
) -> Result<(Vec<VideoTrack>, Vec<AudioTrack>)> {
    let url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={bvid}&cid={cid}&qn={qn}&fnval=1&platform=html5&high_quality=1"
    );
    let value = client.api().get_json::<serde_json::Value>(&url).await?;
    let (videos, audios) = client.parse_tracks(&value)?;

    if !videos.is_empty() || !audios.is_empty() {
        return Ok((videos, audios));
    }

    // DASH 未命中, fallback 到 durl 合并流
    let quality_desc = value["data"]["quality_desc"]
        .as_str()
        .map_or_else(|| format!("Q{qn}"), String::from);
    let durl_arr = value["data"]["durl"].as_array();
    let urls: Vec<String> = durl_arr
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item["url"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let total_size: u64 = durl_arr.map_or(0, |arr| {
        arr.iter().filter_map(|item| item["size"].as_u64()).sum()
    });

    if urls.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    Ok((
        vec![VideoTrack {
            id: format!("html5-{qn}"),
            quality: qn,
            quality_desc,
            codec: "flv".to_string(),
            frame_rate: 0.0,
            resolution: String::new(),
            bandwidth: 0,
            is_hdr: false,
            is_dolby_vision: false,
            is_high_frame_rate: false,
            is_combined: true,
            urls,
            size: total_size,
        }],
        Vec::new(),
    ))
}
