//! playurl 统一客户端.

use anyhow::Result;
use rbd_core::{AudioTrack, BilibiliApi, SubtitleTrack, VideoTrack};

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
    #[allow(clippy::too_many_lines)]
    pub fn parse_tracks(
        &self,
        value: &serde_json::Value,
    ) -> Result<(Vec<VideoTrack>, Vec<AudioTrack>)> {
        // Raw JSON 直接提取视频轨, bypass struct deserialization
        let dash_v = &value["data"]["dash"]["video"];
        let dash_a = &value["data"]["dash"]["audio"];
        let mut videos: Vec<VideoTrack> = Vec::new();
        let mut audios: Vec<AudioTrack> = Vec::new();

        if let Some(v_arr) = dash_v.as_array() {
            for item in v_arr {
                let id = item["id"].as_u64().unwrap_or(0) as u32;
                let base_url = item["baseUrl"]
                    .as_str()
                    .or_else(|| item["base_url"].as_str())
                    .unwrap_or("")
                    .to_string();
                let backup_url: Option<Vec<String>> = item["backupUrl"]
                    .as_array()
                    .or_else(|| item["backup_url"].as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });
                let bandwidth = item["bandwidth"].as_u64().unwrap_or(0);
                let codecid = item["codecid"].as_u64().unwrap_or(0) as u32;
                let codecs = item["codecs"].as_str().unwrap_or("").to_string();
                let width = item["width"].as_u64().unwrap_or(0) as u32;
                let height = item["height"].as_u64().unwrap_or(0) as u32;
                let frame_rate = item["frameRate"]
                    .as_f64()
                    .or_else(|| item["frame_rate"].as_f64())
                    .or_else(|| {
                        item["frameRate"]
                            .as_str()
                            .and_then(|s| s.split('/').next().and_then(|n| n.parse::<f64>().ok()))
                    })
                    .or_else(|| {
                        item["frame_rate"]
                            .as_str()
                            .and_then(|s| s.split('/').next().and_then(|n| n.parse::<f64>().ok()))
                    });
                // 使用原始 JSON 的 accept_description
                let accept_desc: Vec<String> = value["data"]["accept_description"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let qdesc = accept_desc
                    .iter()
                    .find(|d| d.contains(&id.to_string()))
                    .cloned()
                    .unwrap_or_else(|| format!("Q{id}"));

                let track = VideoTrack {
                    id: format!("video-{id}-{codecid}"),
                    quality: id,
                    quality_desc: qdesc,
                    codec: match codecid {
                        7 => "avc".to_string(),
                        12 => "hevc".to_string(),
                        13 => "av1".to_string(),
                        _ => "unknown".to_string(),
                    },
                    frame_rate: frame_rate.unwrap_or(0.0) as f32,
                    resolution: format!("{width}x{height}"),
                    bandwidth,
                    is_hdr: codecs.to_ascii_lowercase().contains("hdr"),
                    is_dolby_vision: codecs.starts_with("dvh1") || codecs.starts_with("dvhe"),
                    is_high_frame_rate: frame_rate.unwrap_or(0.0) >= 50.0,
                    is_combined: false,
                    urls: {
                        let mut urls = Vec::new();
                        if rbd_core::is_safe_download_url(&base_url) {
                            urls.push(base_url);
                        }
                        if let Some(backup) = &backup_url {
                            for url in backup {
                                if rbd_core::is_safe_download_url(url) {
                                    urls.push(url.clone());
                                }
                            }
                        }
                        urls
                    },
                    size: 0,
                };
                videos.push(track);
            }
        }
        if let Some(a_arr) = dash_a.as_array() {
            for item in a_arr {
                let id = item["id"].as_u64().unwrap_or(0) as u32;
                let base_url = item["baseUrl"]
                    .as_str()
                    .or_else(|| item["base_url"].as_str())
                    .unwrap_or("")
                    .to_string();
                let backup_url: Option<Vec<String>> = item["backupUrl"]
                    .as_array()
                    .or_else(|| item["backup_url"].as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });
                let bandwidth = item["bandwidth"].as_u64().unwrap_or(0);
                let codecid = item["codecid"].as_u64().unwrap_or(0) as u32;
                let track = AudioTrack {
                    id: format!("audio-{id}-{codecid}"),
                    quality: id,
                    quality_desc: format!("{}kbps", bandwidth / 1000),
                    codec: match codecid {
                        0 => "eac3".to_string(),
                        1 => "aac".to_string(),
                        2 => "flac".to_string(),
                        3 => "mp3".to_string(),
                        _ => "unknown".to_string(),
                    },
                    bandwidth,
                    is_dolby_atmos: false,
                    is_hi_res: codecid == 2,
                    urls: {
                        let mut urls = Vec::new();
                        if rbd_core::is_safe_download_url(&base_url) {
                            urls.push(base_url);
                        }
                        if let Some(backup) = &backup_url {
                            for url in backup {
                                if rbd_core::is_safe_download_url(url) {
                                    urls.push(url.clone());
                                }
                            }
                        }
                        urls
                    },
                    size: 0,
                };
                audios.push(track);
            }
        }
        Ok((videos, audios))
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
