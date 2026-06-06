//! 颜色计算.
//!
//! B 站弹幕颜色 (0xRRGGBB) ↔ ASS 颜色 (`&H{BBGGRR}&`) 转换,
//! 透明度计算和描边宽度计算.

/// 将 B 站 RGB 整数 (0xRRGGBB) 转换为 ASS BGR 十六进制字符串.
///
/// # 示例
///
/// ```
/// # use rbd_danmaku::color::rgb_to_ass_color;
/// assert_eq!(rgb_to_ass_color(0xFF0000), "0000FF");
/// assert_eq!(rgb_to_ass_color(0x00FF00), "00FF00");
/// ```
#[must_use]
pub fn rgb_to_ass_color(rgb: u32) -> String {
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    format!("{b:02X}{g:02X}{r:02X}")
}

/// 将 B 站不透明度 (0.0-1.0) 转换为 ASS alpha 值 (0-255).
///
/// `alpha = 255 - (opacity * 255)`
/// f32→u8: clamped to [0,1] so result is in [0,255], safe.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn opacity_to_ass_alpha(opacity: f32) -> u8 {
    let o = opacity.clamp(0.0, 1.0);
    (255.0 - o * 255.0) as u8
}

/// 根据字号计算描边宽度.
///
/// `outline = max(fontsize / 25, 1.0)`
#[must_use]
pub fn outline_width(font_size: u8) -> f32 {
    (f32::from(font_size) / 25.0).max(1.0)
}

/// 格式化完整的 ASS 颜色 + 透明度前缀.
///
/// 返回 `\c&H{BBGGRR}&\alpha&H{AA}&`
#[must_use]
pub fn ass_color_alpha_tag(rgb: u32, opacity: f32) -> String {
    let color = rgb_to_ass_color(rgb);
    let alpha = opacity_to_ass_alpha(opacity);
    format!("\\c&H{color}&\\alpha&H{alpha:02X}&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_ass_color_pure_red() {
        assert_eq!(rgb_to_ass_color(0xFF0000), "0000FF");
    }

    #[test]
    fn test_rgb_to_ass_color_pure_green() {
        assert_eq!(rgb_to_ass_color(0x00FF00), "00FF00");
    }

    #[test]
    fn test_rgb_to_ass_color_pure_blue() {
        assert_eq!(rgb_to_ass_color(0x0000FF), "FF0000");
    }

    #[test]
    fn test_rgb_to_ass_color_white() {
        assert_eq!(rgb_to_ass_color(0xFFFFFF), "FFFFFF");
    }

    #[test]
    fn test_rgb_to_ass_color_black() {
        assert_eq!(rgb_to_ass_color(0x000000), "000000");
    }

    #[test]
    fn test_opacity_to_alpha_fully_opaque() {
        assert_eq!(opacity_to_ass_alpha(1.0), 0);
    }

    #[test]
    fn test_opacity_to_alpha_fully_transparent() {
        assert_eq!(opacity_to_ass_alpha(0.0), 255);
    }

    #[test]
    fn test_opacity_to_alpha_half() {
        let alpha = opacity_to_ass_alpha(0.5);
        // 255 - 127 = 128 (allow ±1 rounding)
        assert!((alpha as i32 - 128i32).abs() <= 1);
    }

    #[test]
    fn test_outline_width_min() {
        assert!((outline_width(12) - 1.0).abs() < f32::EPSILON);
        assert!((outline_width(1) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_outline_width_max() {
        let w = outline_width(36);
        assert!((w - 1.44).abs() < 0.001);
    }

    #[test]
    fn test_outline_width_default() {
        let w = outline_width(25);
        assert!((w - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_ass_color_alpha_tag_format() {
        let tag = ass_color_alpha_tag(0xFF0000, 0.8);
        // alpha = 255 - 204 = 51 = 0x33
        assert!(tag.contains("\\c&H0000FF&"));
        assert!(tag.contains("\\alpha&H33&"));
    }
}
