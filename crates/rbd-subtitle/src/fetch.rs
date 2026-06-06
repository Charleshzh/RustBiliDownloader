//! 字幕抓取 — 从 B 站 API 获取字幕列表和内容.
//!
//! **API 来源**:
//! 1. `/x/player/wbi/v2?bvid=xxx&cid=xxx` (主用, 需 WBI)
//! 2. `/x/player/v2?bvid=xxx&cid=xxx` (旧 fallback)
//! 3. `GET` 字幕 URL 直接下载内容

use anyhow::{anyhow, Result};
use rbd_core::BilibiliApi;

use crate::model::{Subtitle, SubtitleFormat};

/// 从字幕 URL 下载原始内容.
///
/// 返回格式和内容字符串.
pub async fn fetch_subtitle_content(url: &str) -> Result<(SubtitleFormat, String)> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await?
        .error_for_status()?;

    let body = response.text().await?;
    let format = crate::format::detect_from_content(&body);

    Ok((format, body))
}

/// 从 WBI v2 API 获取字幕列表.
///
/// 返回字幕信息数组 (不含内容).
pub async fn fetch_subtitle_list(api: &BilibiliApi, bvid: &str, cid: u64) -> Result<Vec<Subtitle>> {
    let value = api.get_subtitles(bvid, cid).await?;
    parse_subtitle_list(&value)
}

/// 解析字幕列表 JSON 响应.
pub fn parse_subtitle_list(value: &serde_json::Value) -> Result<Vec<Subtitle>> {
    let subtitles = value["data"]["subtitle"]["subtitles"]
        .as_array()
        .ok_or_else(|| anyhow!("字幕列表格式错误: 缺少 subtitles 数组"))?;

    let mut result = Vec::new();
    for item in subtitles {
        let raw_url = item["subtitle_url"].as_str().unwrap_or_default();
        let url = if raw_url.starts_with("//") {
            format!("https:{raw_url}")
        } else {
            raw_url.to_string()
        };

        let format = crate::format::detect_from_url(&url);

        result.push(Subtitle {
            id: item["id"]
                .as_i64()
                .map(|id| id.to_string())
                .or_else(|| item["id_str"].as_str().map(ToString::to_string))
                .unwrap_or_else(|| "unknown".to_string()),
            lang: item["lan"].as_str().unwrap_or("unknown").to_string(),
            lang_name: item["lan_doc"].as_str().unwrap_or("Unknown").to_string(),
            format,
            url,
            content: String::new(),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subtitle_list_with_sample() {
        let json = serde_json::json!({
            "data": {
                "subtitle": {
                    "subtitles": [
                        {
                            "id": 1,
                            "id_str": "1",
                            "lan": "zh-Hans",
                            "lan_doc": "中文（简体）",
                            "subtitle_url": "//i0.hdslb.com/bfs/subtitle/abc.srt"
                        },
                        {
                            "id": 2,
                            "lan": "en-US",
                            "lan_doc": "English",
                            "subtitle_url": "https://i0.hdslb.com/bfs/subtitle/def.json"
                        }
                    ]
                }
            }
        });

        let list = parse_subtitle_list(&json).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].lang, "zh-Hans");
        assert_eq!(list[1].lang, "en-US");
        assert!(list[0].url.starts_with("https://"));
    }

    #[test]
    fn test_parse_subtitle_list_empty() {
        let json = serde_json::json!({
            "data": {
                "subtitle": {
                    "subtitles": []
                }
            }
        });
        let list = parse_subtitle_list(&json).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_parse_subtitle_list_missing_key() {
        let json = serde_json::json!({
            "data": {}
        });
        let result = parse_subtitle_list(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_subtitle_format_detection_from_url() {
        assert_eq!(
            crate::format::detect_from_url("//example.com/sub.srt"),
            SubtitleFormat::Srt
        );
        assert_eq!(
            crate::format::detect_from_url("https://example.com/sub.json"),
            SubtitleFormat::Json
        );
    }
}
