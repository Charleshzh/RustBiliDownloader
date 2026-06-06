//! Web playurl 端点.

use anyhow::Result;
use rbd_core::api::FNVAL_DASH_ALL;
use rbd_core::{AudioTrack, VideoTrack};

use crate::client::PlayUrlClient;

/// Web playurl 端点.
pub async fn fetch_web(
    client: &PlayUrlClient,
    bvid: &str,
    cid: u64,
    qn: u32,
) -> Result<(Vec<VideoTrack>, Vec<AudioTrack>)> {
    let url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={bvid}&cid={cid}&qn={qn}&fnval={FNVAL_DASH_ALL}&fnver=0&fourk=1"
    );
    let value = client.api().get_json::<serde_json::Value>(&url).await?;
    let (videos, audios) = client.parse_tracks(&value)?;
    Ok((videos, audios))
}
