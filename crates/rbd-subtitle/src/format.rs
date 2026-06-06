//! 字幕格式探测 — 从 URL 后缀或文件内容推断格式.

use crate::model::SubtitleFormat;

/// 从 URL 推断字幕格式.
///
/// 检查 URL 后缀: `.srt` → Srt, `.ass` / `.ssa` → Ass, `.json` → Json.
/// 不含可识别后缀时默认返回 Json (B 站默认).
#[must_use]
#[allow(clippy::case_sensitive_file_extension_comparisons)]
pub fn detect_from_url(url: &str) -> SubtitleFormat {
    let lower = url.to_lowercase();
    // 去除查询参数
    let path = lower.split('?').next().unwrap_or(&lower);
    if path.ends_with(".srt") {
        SubtitleFormat::Srt
    } else if path.ends_with(".ass") || path.ends_with(".ssa") {
        SubtitleFormat::Ass
    } else if path.ends_with(".json") {
        SubtitleFormat::Json
    } else {
        // B 站默认是 JSON 格式
        SubtitleFormat::Json
    }
}

/// 从内容嗅探字幕格式.
///
/// 检查首字符:
/// - JSON 格式以 `{` 开头
/// - SRT 格式以数字 `1` 开头 (通常)
/// - ASS 格式以 `[Script Info]` 开头
#[must_use]
pub fn detect_from_content(content: &str) -> SubtitleFormat {
    let trimmed = content.trim();

    if trimmed.starts_with('{') {
        SubtitleFormat::Json
    } else if trimmed.starts_with("[Script Info]") {
        SubtitleFormat::Ass
    } else {
        // 默认按 SRT 处理
        SubtitleFormat::Srt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_from_url_srt() {
        assert_eq!(
            detect_from_url("https://example.com/sub.srt"),
            SubtitleFormat::Srt
        );
        assert_eq!(
            detect_from_url("https://example.com/sub.srt?key=val"),
            SubtitleFormat::Srt
        );
    }

    #[test]
    fn test_detect_from_url_ass() {
        assert_eq!(
            detect_from_url("https://example.com/sub.ass"),
            SubtitleFormat::Ass
        );
        assert_eq!(
            detect_from_url("https://example.com/sub.ssa"),
            SubtitleFormat::Ass
        );
    }

    #[test]
    fn test_detect_from_url_json() {
        assert_eq!(
            detect_from_url("https://example.com/sub.json"),
            SubtitleFormat::Json
        );
    }

    #[test]
    fn test_detect_from_url_default() {
        assert_eq!(
            detect_from_url("https://example.com/subtitle"),
            SubtitleFormat::Json
        );
    }

    #[test]
    fn test_detect_from_content_srt() {
        let content = "1\r\n00:00:00,000 --> 00:00:01,000\r\nHello world\r\n";
        assert_eq!(detect_from_content(content), SubtitleFormat::Srt);
    }

    #[test]
    fn test_detect_from_content_json() {
        let content = r#"{"body": [{"from": 0, "to": 1, "content": "Hi"}]}"#;
        assert_eq!(detect_from_content(content), SubtitleFormat::Json);
    }

    #[test]
    fn test_detect_from_content_ass() {
        let content = "[Script Info]\nTitle: Test\n";
        assert_eq!(detect_from_content(content), SubtitleFormat::Ass);
    }
}
