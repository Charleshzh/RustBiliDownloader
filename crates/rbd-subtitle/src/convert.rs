//! 字幕格式转换 — JSON ↔ SRT ↔ ASS.
//!
//! **算法来源**: BBDown/SubUtil.cs `JsonSub2Srt` + Yutto `ass.py`.

use std::fmt::Write;

use anyhow::{anyhow, Result};

use crate::model::SubtitleBody;

/// 将 B 站 JSON 字幕转为 SRT 格式.
///
/// B 站 JSON 格式: `{"body": [{"from": 0.5, "to": 2.0, "content": "文本"}, ...]}`
/// 输出标准 SRT:
/// ```text
/// 1
/// 00:00:00,500 --> 00:00:02,000
/// 文本
///
/// 2
/// ...
/// ```
pub fn json_to_srt(json_content: &str) -> Result<String> {
    let body: SubtitleBody = serde_json::from_str(json_content)
        .map_err(|err| anyhow!("解析 B 站 JSON 字幕失败: {err}"))?;

    if body.body.is_empty() {
        return Ok(String::new());
    }

    let mut srt = String::new();
    for (i, entry) in body.body.iter().enumerate() {
        let seq = i + 1;
        let start = format_timestamp(entry.from);
        let end = format_timestamp(entry.to);
        let content = &entry.content;

        let _ = write!(srt, "{seq}\r\n");
        let _ = write!(srt, "{start} --> {end}\r\n");
        let _ = write!(srt, "{content}\r\n\r\n");
    }

    Ok(srt)
}

/// 将 SRT 字幕转为 ASS 格式.
///
/// 添加标准 ASS 头部, 保留原 SRT 内容.
pub fn srt_to_ass(srt_content: &str) -> Result<String> {
    let events = parse_srt_events(srt_content)?;

    let mut ass = String::new();
    // ASS header
    ass.push_str("[Script Info]\r\n");
    ass.push_str("Title: Converted from SRT\r\n");
    ass.push_str("ScriptType: v4.00+\r\n");
    ass.push_str("WrapStyle: 0\r\n");
    ass.push_str("PlayResX: 1920\r\n");
    ass.push_str("PlayResY: 1080\r\n");
    ass.push_str("\r\n");
    ass.push_str("[V4+ Styles]\r\n");
    ass.push_str("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, ");
    ass.push_str("OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, ");
    ass.push_str("Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, ");
    ass.push_str("MarginV, Encoding\r\n");
    ass.push_str(
        "Style: Default,Microsoft YaHei,42,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,30,30,15,1\r\n",
    );
    ass.push_str("\r\n");
    ass.push_str("[Events]\r\n");
    ass.push_str("Format: Layer, Start, End, Style, Name, MarginL, MarginR, ");
    ass.push_str("MarginV, Effect, Text\r\n");

    for event in events {
        let start = format_ass_timestamp(event.start);
        let end = format_ass_timestamp(event.end);
        let _ = write!(ass, "Dialogue: 0,{start},{end},Default,,0,0,0,,{}\r\n", event.text);
    }

    Ok(ass)
}

/// 将 ASS 字幕转为 SRT 格式.
///
/// 剥离 ASS 头部和样式, 仅提取 Dialogue 行.
pub fn ass_to_srt(ass_content: &str) -> Result<String> {
    let events = parse_ass_events(ass_content);

    let mut srt = String::new();
    for (i, event) in events.iter().enumerate() {
        let seq = i + 1;
        let start = format_timestamp(event.start);
        let end = format_timestamp(event.end);
        let _ = write!(srt, "{seq}\r\n");
        let _ = write!(srt, "{start} --> {end}\r\n");
        let _ = write!(srt, "{}\r\n\r\n", event.text);
    }

    Ok(srt)
}

// ── helpers ──────────────────────────────────────────────

struct SubEvent {
    start: f64,
    end: f64,
    text: String,
}

/// 格式化时间戳: `00:00:01,500` (SRT 用逗号分隔毫秒).
fn format_timestamp(seconds: f64) -> String {
    let total_ms = (seconds * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let secs = total_secs % 60;
    let total_mins = total_secs / 60;
    let mins = total_mins % 60;
    let hours = total_mins / 60;
    format!("{hours:02}:{mins:02}:{secs:02},{ms:03}")
}

/// 格式化 ASS 时间戳: `0:00:01.50`.
fn format_ass_timestamp(seconds: f64) -> String {
    let total_cs = (seconds * 100.0).round() as u64;
    let cs = total_cs % 100;
    let total_secs = total_cs / 100;
    let secs = total_secs % 60;
    let total_mins = total_secs / 60;
    let mins = total_mins % 60;
    let hours = total_mins / 60;
    format!("{hours}:{mins:02}:{secs:02}.{cs:02}")
}

/// 解析 SRT 内容提取事件列表.
fn parse_srt_events(srt: &str) -> Result<Vec<SubEvent>> {
    let mut events = Vec::new();
    let mut lines = srt.lines().peekable();

    while let Some(seq_line) = lines.next() {
        // skip empty lines
        if seq_line.trim().is_empty() {
            continue;
        }

        // expect timestamp line
        let ts_line = lines
            .next()
            .ok_or_else(|| anyhow!("SRT 格式错误: 缺少时间戳行"))?;
        let ts_line = ts_line.trim();

        let parts: Vec<&str> = ts_line.split("-->").collect();
        if parts.len() < 2 {
            return Err(anyhow!("SRT 格式错误: 无效时间戳 '{ts_line}'"));
        }

        let start = parse_srt_time(parts[0].trim())?;
        let end = parse_srt_time(parts[1].trim())?;

        let mut text = String::new();
        for content_line in lines.by_ref() {
            let trimmed = content_line.trim();
            if trimmed.is_empty() {
                break;
            }
            if !text.is_empty() {
                text.push_str("\\N");
            }
            text.push_str(trimmed);
        }

        events.push(SubEvent { start, end, text });
    }

    Ok(events)
}

/// 解析 SRT 时间字符串 "00:00:01,500" -> 1.5 秒.
fn parse_srt_time(s: &str) -> Result<f64> {
    // 格式: HH:MM:SS,mmm 或 HH:MM:SS.mmm
    let s = s.replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(anyhow!("无效 SRT 时间格式: '{s}'"));
    }
    let hours: f64 = parts[0]
        .parse()
        .map_err(|_| anyhow!("无法解析小时: '{}'", parts[0]))?;
    let mins: f64 = parts[1]
        .parse()
        .map_err(|_| anyhow!("无法解析分钟: '{}'", parts[1]))?;
    let secs: f64 = parts[2]
        .parse()
        .map_err(|_| anyhow!("无法解析秒: '{}'", parts[2]))?;
    Ok(hours * 3600.0 + mins * 60.0 + secs)
}

/// 解析 ASS 文件提取 Dialogue 事件.
fn parse_ass_events(ass: &str) -> Vec<SubEvent> {
    let mut events = Vec::new();
    let mut in_events = false;

    for line in ass.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[Events]") {
            in_events = true;
            continue;
        }
        if trimmed.starts_with('[') && in_events {
            break; // next section
        }
        if !in_events {
            continue;
        }
        if trimmed.to_lowercase().starts_with("dialogue:") {
            if let Some(event) = parse_ass_dialogue(trimmed) {
                events.push(event);
            }
        }
    }

    // 函数 parse_ass_events 现在直接返回 Vec<SubEvent>
    events
}

/// 解析单行 ASS Dialogue.
fn parse_ass_dialogue(line: &str) -> Option<SubEvent> {
    // 格式: Dialogue: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text
    let after_dialogue = line.strip_prefix("Dialogue:")?;
    let parts: Vec<&str> = after_dialogue.splitn(10, ',').collect();
    if parts.len() < 10 {
        return None;
    }
    let start = parse_ass_time(parts[1].trim())?;
    let end = parse_ass_time(parts[2].trim())?;
    let text = clean_ass_text(parts[9]);
    Some(SubEvent { start, end, text })
}

/// 解析 ASS 时间 "0:01:23.45" -> 83.45 秒.
fn parse_ass_time(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let hours: f64 = parts[0].parse().ok()?;
    let mins: f64 = parts[1].parse().ok()?;
    let secs: f64 = parts[2].parse().ok()?;
    Some(hours * 3600.0 + mins * 60.0 + secs)
}

/// 清理 ASS 文本中的样式标签.
fn clean_ass_text(text: &str) -> String {
    // 移除 {\xxx} 样式标签
    let mut result = String::new();
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '{' => in_tag = true,
            '}' if in_tag => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_srt_basic() {
        let json = r#"{
            "font_size": 0.4,
            "body": [
                {"from": 0.5, "to": 2.0, "content": "Hello World", "location": 0}
            ]
        }"#;
        let srt = json_to_srt(json).unwrap();
        assert!(srt.contains("00:00:00,500"));
        assert!(srt.contains("00:00:02,000"));
        assert!(srt.contains("Hello World"));
    }

    #[test]
    fn test_json_to_srt_multiple() {
        let json = r#"{
            "body": [
                {"from": 0.0, "to": 1.0, "content": "First", "location": 0},
                {"from": 1.0, "to": 3.0, "content": "Second", "location": 0}
            ]
        }"#;
        let srt = json_to_srt(json).unwrap();
        assert!(srt.contains("1\r\n"));
        assert!(srt.contains("2\r\n"));
    }

    #[test]
    fn test_json_to_srt_empty() {
        let json = r#"{"body": []}"#;
        let srt = json_to_srt(json).unwrap();
        assert!(srt.is_empty());
    }

    #[test]
    fn test_json_to_srt_invalid() {
        let result = json_to_srt("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_srt_to_ass_basic() {
        let srt = "1\r\n00:00:00,500 --> 00:00:02,000\r\nHello\r\n\r\n";
        let ass = srt_to_ass(srt).unwrap();
        assert!(ass.contains("[Script Info]"));
        assert!(ass.contains("[V4+ Styles]"));
        assert!(ass.contains("[Events]"));
        assert!(ass.contains("Hello"));
    }

    #[test]
    fn test_ass_to_srt_basic() {
        let ass = r#"[Script Info]
Title: Test

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour
Style: Default,Microsoft YaHei,42

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:00.50,0:00:02.00,Default,,0,0,0,,Hello World
"#;
        let srt = ass_to_srt(ass).unwrap();
        assert!(srt.contains("00:00:00,500"));
        assert!(srt.contains("00:00:02,000"));
        assert!(srt.contains("Hello World"));
    }

    #[test]
    fn test_parse_srt_time() {
        let ts = parse_srt_time("00:01:30,500").unwrap();
        assert!((ts - 90.5).abs() < 0.01);
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(90.5), "00:01:30,500");
        assert_eq!(format_timestamp(3661.123), "01:01:01,123");
    }

    #[test]
    fn test_format_ass_timestamp() {
        assert_eq!(format_ass_timestamp(90.5), "0:01:30.50");
        assert_eq!(format_ass_timestamp(3661.12), "1:01:01.12");
    }

    #[test]
    fn test_clean_ass_text() {
        assert_eq!(
            clean_ass_text(r"{\pos(100,200)}Hello{\b1} World"),
            "Hello World"
        );
        assert_eq!(clean_ass_text("Plain text"), "Plain text");
    }
}
