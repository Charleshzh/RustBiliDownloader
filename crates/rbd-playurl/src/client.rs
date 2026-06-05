//! playurl 统一客户端.

use anyhow::Result;
use rbd_core::{parse_playurl, AudioTrack, BilibiliApi, PlayUrlResponse, SubtitleTrack, VideoTrack};

/// playurl 统一客户端.
pub struct PlayUrlClient {
    api: BilibiliApi,
}

impl PlayUrlClient {
    /// 创建客户端.
    #[must_use]
    pub fn new(api: BilibiliApi) -> Self {
        Self { api }
    }

    /// 访问底层 API.
    #[must_use]
    pub fn api(&self) -> &BilibiliApi {
        &self.api
    }

    /// 解析视频/音频轨.
    pub fn parse_tracks(
        &self,
        value: &serde_json::Value,
    ) -> Result<(Vec<VideoTrack>, Vec<AudioTrack>, PlayUrlResponse)> {
        let parsed = parse_playurl(value)?;
        let videos = parsed.data.clone().into_tracks();
        let audios = parsed.data.clone().into_audio_tracks();
        Ok((videos, audios, parsed))
    }

    /// 获取字幕轨.
    pub async fn fetch_subtitles(&self, bvid: &str, cid: u64) -> Result<Vec<SubtitleTrack>> {
        let value = self.api.get_subtitles(bvid, cid).await?;
        Ok(parse_subtitles(&value))
    }
}

fn parse_subtitles(value: &serde_json::Value) -> Vec<SubtitleTrack> {
    value["data"]["subtitle"]["subtitles"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let raw_url = item
                        .get("subtitle_url")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    let url = if raw_url.starts_with("//") {
                        format!("https:{raw_url}")
                    } else {
                        raw_url.to_string()
                    };
                    SubtitleTrack {
                        id: item
                            .get("id")
                            .and_then(serde_json::Value::as_i64)
                            .map_or_else(|| "subtitle".to_string(), |id| id.to_string()),
                        lang: item
                            .get("lan")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                        lang_name: item
                            .get("lan_doc")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Unknown")
                            .to_string(),
                        format: subtitle_format(&url),
                        url,
                        source: item
                            .get("id_str")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        is_ai: item
                            .get("ai_type")
                            .and_then(serde_json::Value::as_i64)
                            .unwrap_or_default()
                            > 0,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn subtitle_format(url: &str) -> String {
    url.rsplit('.')
        .next()
        .filter(|suffix| suffix.len() <= 5)
        .unwrap_or("json")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::subtitle_format;

    #[test]
    fn test_subtitle_format_defaults_to_json() {
        assert_eq!(subtitle_format("https://example.com/subtitle"), "json");
        assert_eq!(subtitle_format("https://example.com/subtitle.srt"), "srt");
    }
}
