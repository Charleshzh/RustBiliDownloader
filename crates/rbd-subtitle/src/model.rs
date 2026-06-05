//! 字幕数据模型.
//!
//! 包含字幕元信息和内容, 支持 JSON / SRT / ASS 三种格式.

use serde::{Deserialize, Serialize};

/// 字幕格式.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleFormat {
    /// SubRip (.srt).
    Srt,
    /// Advanced Substation Alpha (.ass / .ssa).
    Ass,
    /// B 站 JSON 格式.
    Json,
}

/// 字幕条目 (B 站 JSON 格式).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleEntry {
    /// 起始时间 (秒).
    pub from: f64,
    /// 结束时间 (秒).
    pub to: f64,
    /// 字幕内容.
    pub content: String,
    /// 位置 (顶部/底部), 0=底部.
    #[serde(default)]
    pub location: u32,
}

/// B 站 JSON 字幕顶层包装.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleBody {
    /// 字体大小.
    #[serde(default)]
    pub font_size: f64,
    /// 字幕条目.
    pub body: Vec<SubtitleEntry>,
}

/// 字幕信息 (下载前).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtitle {
    /// 字幕 ID (来自 API).
    pub id: String,
    /// 语言代码 (e.g. "zh-Hans", "en-US").
    pub lang: String,
    /// 语言名称 (e.g. "简体中文", "English").
    pub lang_name: String,
    /// 字幕格式.
    pub format: SubtitleFormat,
    /// 下载 URL.
    pub url: String,
    /// 字幕原始内容 (下载后填充).
    #[serde(default)]
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtitle_format_serde() {
        let json = r#""srt""#;
        let fmt: SubtitleFormat = serde_json::from_str(json).unwrap();
        assert_eq!(fmt, SubtitleFormat::Srt);

        let json = r#""ass""#;
        let fmt: SubtitleFormat = serde_json::from_str(json).unwrap();
        assert_eq!(fmt, SubtitleFormat::Ass);

        let json = r#""json""#;
        let fmt: SubtitleFormat = serde_json::from_str(json).unwrap();
        assert_eq!(fmt, SubtitleFormat::Json);
    }

    #[test]
    fn test_subtitle_entry_deserialize() {
        let json = r#"{"from": 1.5, "to": 3.0, "content": "Hello", "location": 0}"#;
        let entry: SubtitleEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.from, 1.5);
        assert_eq!(entry.to, 3.0);
        assert_eq!(entry.content, "Hello");
    }

    #[test]
    fn test_subtitle_body_deserialize() {
        let json = r#"{
            "font_size": 0.4,
            "body": [
                {"from": 0.0, "to": 2.0, "content": "First", "location": 0},
                {"from": 2.0, "to": 5.0, "content": "Second", "location": 0}
            ]
        }"#;
        let body: SubtitleBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.body.len(), 2);
        assert_eq!(body.body[0].content, "First");
    }

    #[test]
    fn test_subtitle_serde() {
        let sub = Subtitle {
            id: "1".to_string(),
            lang: "zh-Hans".to_string(),
            lang_name: "简体中文".to_string(),
            format: SubtitleFormat::Srt,
            url: "https://example.com/sub.srt".to_string(),
            content: String::new(),
        };
        let json = serde_json::to_string(&sub).unwrap();
        let parsed: Subtitle = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "1");
        assert_eq!(parsed.lang, "zh-Hans");
    }

    #[test]
    fn test_subtitle_content_field() {
        let mut sub = Subtitle {
            id: "1".to_string(),
            lang: "en-US".to_string(),
            lang_name: "English".to_string(),
            format: SubtitleFormat::Ass,
            url: "https://example.com/sub.ass".to_string(),
            content: String::new(),
        };
        assert!(sub.content.is_empty());
        sub.content = "1\r\n00:00:00,000 --> 00:00:01,000\r\nHello\r\n".to_string();
        assert!(!sub.content.is_empty());
    }
}
