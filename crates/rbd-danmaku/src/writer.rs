//! 弹幕输出 — ASS 格式写入.
//!
//! 生成 ASS 文件头部和每条弹幕的 Dialogue 行.

/// 生成 ASS 文件头部.
#[must_use]
pub fn write_head(
    video_width: u32,
    video_height: u32,
    font_name: &str,
    font_size: u8,
    opacity: f32,
) -> String {
    let alpha = crate::color::opacity_to_ass_alpha(opacity);
    let outline = crate::color::outline_width(font_size);
    format!(
        "[Script Info]\n\
        ScriptType: V4.00+\n\
        PlayResX: {video_width}\n\
        PlayResY: {video_height}\n\
        \n\
        [V4+ Styles]\n\
        Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
        Style: Default,{font_name},{font_size},&H{alpha:02X}FFFFFF,&H000000FF,&H00000000,&H00000000,1,0,0,0,100,100,0,0,1,{outline:.1},0,7,0,0,0,1\n\
        \n\
        [Events]\n\
        Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
    )
}

/// 写入单条弹幕的 Dialogue 行.
///
/// 根据 mode 生成对应的定位标签:
/// - 模式 4 (底部): `\an8\pos(x, y)`
/// - 模式 5 (顶部): `\an2\pos(x, y)`
/// - 模式 1 (滚动): `\move(x1, y1, x2, y2)`
#[must_use]
pub fn write_danmaku(
    time: f32,
    duration: f32,
    mode: u8,
    rgb: u32,
    content: &str,
    line: u32,
) -> String {
    let start = format_ass_time(time);
    let end = format_ass_time(time + duration);
    let color_tag = crate::color::ass_color_alpha_tag(rgb, 0.8);
    let pos_tag = match mode {
        4 => format!(
            r"\an8\pos({},{})",
            0,
            (i32::try_from(line).unwrap_or(0) + 1) * 30
        ),
        5 => format!(
            r"\an2\pos({},{})",
            0,
            (i32::try_from(line).unwrap_or(0) + 1) * 30
        ),
        _ => {
            // 默认滚动: 从右边缘滚出到左边缘外
            format!(
                r"\move({},{},{},{})",
                1920,
                (i32::try_from(line).unwrap_or(0) + 1) * 30,
                -100,
                (i32::try_from(line).unwrap_or(0) + 1) * 30
            )
        }
    };
    format!("Dialogue: 0,{start},{end},Default,,0,0,0,,{pos_tag}{color_tag}{content}\n")
}

/// 格式化 ASS 时间戳, f32→u32: t 非负, 值在合理范围内.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn format_ass_time(t: f32) -> String {
    let h = (t / 3600.0) as u32;
    let m = ((t % 3600.0) / 60.0) as u32;
    let s = (t % 60.0) as u32;
    let cs = (t * 100.0) as u32 % 100;
    format!("{h}:{m:02}:{s:02}.{cs:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ass_time_zero() {
        assert_eq!(format_ass_time(0.0), "0:00:00.00");
    }

    #[test]
    fn test_format_ass_time_one_hour() {
        assert_eq!(format_ass_time(3600.0), "1:00:00.00");
    }

    #[test]
    fn test_format_ass_time_complex() {
        // 3725.5s = 1h 2m 5.5s → "1:02:05.50"
        assert_eq!(format_ass_time(3725.5), "1:02:05.50");
    }

    #[test]
    fn test_write_head_contains_required_sections() {
        let head = write_head(1920, 1080, "SimHei", 25, 0.8);
        assert!(head.contains("[Script Info]"));
        assert!(head.contains("[V4+ Styles]"));
        assert!(head.contains("[Events]"));
        assert!(head.contains("Style: Default,SimHei,25"));
    }

    #[test]
    fn test_write_head_uses_video_dimensions() {
        let head = write_head(1280, 720, "Arial", 16, 1.0);
        assert!(head.contains("PlayResX: 1280"));
        assert!(head.contains("PlayResY: 720"));
        assert!(head.contains("Style: Default,Arial,16"));
    }

    #[test]
    fn test_write_danmaku_scroll() {
        let line = write_danmaku(10.0, 8.0, 1, 0xFFFFFF, "test", 0);
        // 滚动模式应该包含 \move
        assert!(line.contains("\\move("));
        // 时间格式
        assert!(line.contains("0:00:10.00"));
        assert!(line.contains("0:00:18.00"));
    }

    #[test]
    fn test_write_danmaku_top() {
        let line = write_danmaku(0.0, 5.0, 5, 0xFF0000, "top", 0);
        // 顶部模式应该包含 \an2
        assert!(line.contains("\\an2"));
    }

    #[test]
    fn test_write_danmaku_bottom() {
        let line = write_danmaku(0.0, 5.0, 4, 0x00FF00, "bottom", 0);
        // 底部模式应该包含 \an8
        assert!(line.contains("\\an8"));
    }
}
