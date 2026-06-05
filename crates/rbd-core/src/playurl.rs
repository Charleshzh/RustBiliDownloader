//! playurl 响应解析 (DASH / durl).

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::{AudioTrack, VideoTrack};

/// playurl 顶层响应.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayUrlResponse {
    /// 错误码.
    pub code: i32,
    /// 返回消息.
    pub message: String,
    /// 实际数据.
    pub data: PlayUrlData,
}

/// playurl 数据体.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayUrlData {
    /// 当前画质.
    pub quality: u32,
    /// 画质描述.
    pub quality_desc: Option<String>,
    /// 返回格式.
    pub format: Option<String>,
    /// 总时长 (ms).
    pub timelength: u64,
    /// 可选画质描述.
    pub accept_description: Vec<String>,
    /// 可选画质代码.
    pub accept_quality: Vec<u32>,
    /// DASH 结构.
    pub dash: Option<Dash>,
    /// FLV/durl 结构.
    pub durl: Option<Vec<Durl>>,
}

/// DASH 信息.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Dash {
    /// 时长.
    pub duration: u32,
    /// 视频流.
    pub video: Vec<DashVideo>,
    /// 音频流.
    pub audio: Option<Vec<DashAudio>>,
    /// 杜比音频.
    pub dolby: Option<Dolby>,
    /// Hi-Res FLAC.
    pub flac: Option<Flac>,
}

/// DASH 视频流.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DashVideo {
    /// 质量 ID.
    pub id: u32,
    /// 主 URL.
    #[serde(alias = "baseUrl")]
    pub base_url: String,
    /// 备份 URL.
    #[serde(alias = "backupUrl")]
    pub backup_url: Option<Vec<String>>,
    /// 比特率.
    pub bandwidth: u64,
    /// MIME 类型.
    #[serde(alias = "mimeType")]
    pub mime_type: String,
    /// codec 字符串.
    pub codecs: String,
    /// 宽度.
    pub width: u32,
    /// 高度.
    pub height: u32,
    /// 帧率.
    #[serde(deserialize_with = "deserialize_frame_rate")]
    pub frame_rate: Option<f32>,
    /// B 站 codec id.
    pub codecid: u32,
}

/// DASH 音频流.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DashAudio {
    /// 质量 ID.
    pub id: u32,
    /// 主 URL.
    #[serde(alias = "baseUrl")]
    pub base_url: String,
    /// 备份 URL.
    #[serde(alias = "backupUrl")]
    pub backup_url: Option<Vec<String>>,
    /// 比特率.
    pub bandwidth: u64,
    /// MIME 类型.
    #[serde(alias = "mimeType")]
    pub mime_type: String,
    /// codec 字符串.
    pub codecs: String,
    /// B 站 codec id.
    pub codecid: u32,
}

/// 杜比音频信息.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Dolby {
    /// 杜比音轨.
    pub audio: Option<Vec<DashAudio>>,
}

/// FLAC 信息.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Flac {
    /// FLAC 音轨.
    pub audio: Option<DashAudio>,
}

/// durl 响应.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Durl {
    /// URL.
    pub url: String,
    /// 文件大小.
    pub size: u64,
    /// 时长.
    pub length: u64,
}

/// 解析 playurl 响应.
pub fn parse_playurl(value: &serde_json::Value) -> Result<PlayUrlResponse> {
    Ok(serde_json::from_value(value.clone())?)
}

impl PlayUrlData {
    /// 转为视频轨.
    pub fn into_tracks(self) -> Vec<VideoTrack> {
        let quality_descs = accept_description_to_quality_desc(&self.accept_description);
        self.dash
            .map(|dash| {
                dash.video
                    .into_iter()
                    .map(|video| {
                        let quality_desc = quality_descs
                            .iter()
                            .find(|desc| desc_contains_quality(desc, video.id))
                            .cloned()
                            .or_else(|| self.quality_desc.clone())
                            .unwrap_or_else(|| format!("Q{}", video.id));
                        let frame_rate = video.frame_rate.unwrap_or(0.0);
                        let codecs = video.codecs.clone();
                        VideoTrack {
                            id: format!("video-{}-{}", video.id, video.codecid),
                            quality: video.id,
                            quality_desc,
                            codec: map_video_codec(video.codecid),
                            frame_rate,
                            resolution: format!("{}x{}", video.width, video.height),
                            bandwidth: video.bandwidth,
                            is_hdr: codecs.to_ascii_lowercase().contains("hdr"),
                            is_dolby_vision: codecs.starts_with("dvh1") || codecs.starts_with("dvhe"),
                            is_high_frame_rate: frame_rate >= 50.0,
                            is_combined: false,
                            urls: collect_urls(video.base_url, video.backup_url),
                            size: 0,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 转为音频轨.
    pub fn into_audio_tracks(self) -> Vec<AudioTrack> {
        let mut tracks = Vec::new();
        if let Some(dash) = self.dash {
            if let Some(audio) = dash.audio {
                tracks.extend(audio.into_iter().map(|track| into_audio_track(track, false, false)));
            }
            if let Some(dolby) = dash.dolby.and_then(|dolby| dolby.audio) {
                tracks.extend(dolby.into_iter().map(|track| into_audio_track(track, true, false)));
            }
            if let Some(flac) = dash.flac.and_then(|flac| flac.audio) {
                tracks.push(into_audio_track(flac, false, true));
            }
        }
        tracks
    }
}

/// 将 accept_description 规格化为描述列表.
pub fn accept_description_to_quality_desc(accept: &[String]) -> Vec<String> {
    accept.iter().map(|item| item.trim().to_string()).collect()
}

fn map_video_codec(codecid: u32) -> String {
    match codecid {
        7 => "avc",
        12 => "hevc",
        13 => "av1",
        _ => "unknown",
    }
    .to_string()
}

fn map_audio_codec(codecid: u32) -> String {
    match codecid {
        0 => "eac3",
        1 => "aac",
        2 => "flac",
        3 => "mp3",
        _ => "unknown",
    }
    .to_string()
}

fn into_audio_track(audio: DashAudio, is_dolby_atmos: bool, is_hi_res: bool) -> AudioTrack {
    let quality_desc = if is_hi_res {
        "Hi-Res".to_string()
    } else if is_dolby_atmos {
        "Dolby Atmos".to_string()
    } else {
        format!("{}kbps", audio.bandwidth / 1000)
    };

    AudioTrack {
        id: format!("audio-{}-{}", audio.id, audio.codecid),
        quality: audio.id,
        quality_desc,
        codec: map_audio_codec(audio.codecid),
        bandwidth: audio.bandwidth,
        is_dolby_atmos,
        is_hi_res,
        urls: collect_urls(audio.base_url, audio.backup_url),
        size: 0,
    }
}

fn collect_urls(base_url: String, backup_url: Option<Vec<String>>) -> Vec<String> {
    let mut urls = Vec::new();
    if crate::is_safe_download_url(&base_url) {
        urls.push(base_url);
    } else {
        tracing::warn!("跳过不安全的下载 URL: {base_url}");
    }
    if let Some(backup) = backup_url {
        for url in backup {
            if crate::is_safe_download_url(&url) {
                urls.push(url);
            } else {
                tracing::warn!("跳过不安全的备份 URL: {url}");
            }
        }
    }
    urls
}

fn desc_contains_quality(desc: &str, quality: u32) -> bool {
    desc.contains(&quality.to_string())
        || match quality {
            16 => desc.contains("360"),
            32 => desc.contains("480"),
            64 => desc.contains("720"),
            80 => desc.contains("1080"),
            120 => desc.contains("4K"),
            127 => desc.contains("8K"),
            _ => false,
        }
}

fn deserialize_frame_rate<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FrameRate {
        Number(f32),
        Text(String),
        Null,
    }

    Ok(match Option::<FrameRate>::deserialize(deserializer)? {
        Some(FrameRate::Number(v)) => Some(v),
        Some(FrameRate::Text(v)) => v.split('/').next().and_then(|s| s.parse::<f32>().ok()),
        Some(FrameRate::Null) | None => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dash_response() {
        let value = serde_json::json!({
            "code": 0,
            "message": "0",
            "data": {
                "quality": 80,
                "quality_desc": "1080P",
                "format": "dash",
                "timelength": 1000,
                "accept_description": ["360P", "1080P"],
                "accept_quality": [16, 80],
                "dash": {
                    "duration": 100,
                    "video": [{
                        "id": 80,
                        "base_url": "https://video/main.m4s",
                        "backup_url": ["https://video/backup.m4s"],
                        "bandwidth": 2330000,
                        "mime_type": "video/mp4",
                        "codecs": "hev1.1.6.L120.90",
                        "width": 1920,
                        "height": 1080,
                        "frame_rate": "60",
                        "codecid": 12
                    }],
                    "audio": [{
                        "id": 30280,
                        "base_url": "https://audio/main.m4s",
                        "backup_url": ["https://audio/backup.m4s"],
                        "bandwidth": 192000,
                        "mime_type": "audio/mp4",
                        "codecs": "mp4a.40.2",
                        "codecid": 1
                    }]
                }
            }
        });

        let parsed = parse_playurl(&value).unwrap();
        let videos = parsed.data.clone().into_tracks();
        let audios = parsed.data.into_audio_tracks();

        assert_eq!(parsed.code, 0);
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].codec, "hevc");
        assert!(videos[0].is_high_frame_rate);
        assert_eq!(videos[0].urls.len(), 2);
        assert_eq!(audios.len(), 1);
        assert_eq!(audios[0].codec, "aac");
    }

    #[test]
    fn test_parse_durl_response() {
        let value = serde_json::json!({
            "code": 0,
            "message": "0",
            "data": {
                "quality": 64,
                "timelength": 12345,
                "accept_description": ["480P"],
                "accept_quality": [64],
                "durl": [{
                    "url": "https://flv/main.flv",
                    "size": 100,
                    "length": 12345
                }]
            }
        });

        let parsed = parse_playurl(&value).unwrap();
        assert!(parsed.data.dash.is_none());
        assert_eq!(parsed.data.durl.unwrap()[0].url, "https://flv/main.flv");
    }

    #[test]
    fn test_into_tracks_dolby_vision() {
        let value = serde_json::json!({
            "code": 0,
            "message": "0",
            "data": {
                "quality": 126,
                "timelength": 1000,
                "accept_description": ["杜比视界"],
                "accept_quality": [126],
                "dash": {
                    "duration": 100,
                    "video": [{
                        "id": 126,
                        "base_url": "https://video/dv.m4s",
                        "bandwidth": 4000000,
                        "mime_type": "video/mp4",
                        "codecs": "dvh1.08.01",
                        "width": 3840,
                        "height": 2160,
                        "frame_rate": "24",
                        "codecid": 13
                    }]
                }
            }
        });

        let parsed = parse_playurl(&value).unwrap();
        let tracks = parsed.data.into_tracks();
        assert!(tracks[0].is_dolby_vision);
    }

    #[test]
    fn test_into_tracks_hevc() {
        let value = serde_json::json!({
            "quality": 80,
            "timelength": 1000,
            "accept_description": ["1080P"],
            "accept_quality": [80],
            "dash": {
                "duration": 100,
                "video": [{
                    "id": 80,
                    "base_url": "https://video/main.m4s",
                    "bandwidth": 2330000,
                    "mime_type": "video/mp4",
                    "codecs": "hev1.1.6.L120.90",
                    "width": 1920,
                    "height": 1080,
                    "frame_rate": "30",
                    "codecid": 12
                }]
            }
        });

        let data: PlayUrlData = serde_json::from_value(value).unwrap();
        let tracks = data.into_tracks();
        assert_eq!(tracks[0].codec, "hevc");
    }

    #[test]
    fn test_into_audio_tracks_flac() {
        let value = serde_json::json!({
            "quality": 80,
            "timelength": 1000,
            "accept_description": ["1080P"],
            "accept_quality": [80],
            "dash": {
                "duration": 100,
                "video": [],
                "audio": [{
                    "id": 30280,
                    "base_url": "https://audio/aac.m4s",
                    "bandwidth": 192000,
                    "mime_type": "audio/mp4",
                    "codecs": "mp4a.40.2",
                    "codecid": 1
                }],
                "flac": {
                    "audio": {
                        "id": 30251,
                        "base_url": "https://audio/flac.m4s",
                        "bandwidth": 999000,
                        "mime_type": "audio/flac",
                        "codecs": "fLaC",
                        "codecid": 2
                    }
                }
            }
        });

        let data: PlayUrlData = serde_json::from_value(value).unwrap();
        let tracks = data.into_audio_tracks();
        assert!(tracks.iter().any(|track| track.is_hi_res));
    }
}
