//! 弹幕渲染选项.
//!
//! 控制渲染行为: 字号, 字体, 透明度, 速度, 行数, 过滤规则等.

use crate::model::DanmakuMode;

/// 渲染选项.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// 视频宽度 (像素, 用于计算 position)
    pub video_width: u32,
    /// 视频高度 (像素)
    pub video_height: u32,
    /// 字体名称
    pub font_name: String,
    /// 默认字号 (12-36)
    pub font_size: u8,
    /// 不透明度 (0.0-1.0)
    pub opacity: f32,
    /// 行间距 (像素)
    pub line_spacing: u32,
    /// 滚动速度系数 (1.0 = 默认)
    pub scroll_speed: f32,
    /// 显示时长 (秒) — 滚动弹幕停留时间
    pub display_duration: f32,
    /// 是否显示底部弹幕
    pub show_bottom: bool,
    /// 是否显示顶部弹幕
    pub show_top: bool,
    /// 是否显示滚动弹幕
    pub show_scroll: bool,
    /// 屏蔽关键词 (正则)
    pub block_keyword_patterns: Vec<String>,
    /// 屏蔽模式 (精确按 mode 屏蔽)
    pub block_modes: Vec<DanmakuMode>,
    /// 最大行数 (None = 根据视频高度自动计算)
    pub max_rows: Option<u32>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            video_width: 1920,
            video_height: 1080,
            font_name: "SimHei".to_string(),
            font_size: 25,
            opacity: 0.8,
            line_spacing: 4,
            scroll_speed: 1.0,
            display_duration: 8.0,
            show_bottom: true,
            show_top: true,
            show_scroll: true,
            block_keyword_patterns: vec![],
            block_modes: vec![],
            max_rows: None,
        }
    }
}

impl RenderOptions {
    /// 计算当前视频尺寸下的实际最大行数.
    ///
    /// 如果未设置 `max_rows`, 则根据 `video_height / (font_size + line_spacing)` 计算.
    #[must_use]
    pub fn effective_max_rows(&self) -> u32 {
        self.max_rows.unwrap_or_else(|| {
            let h = self.font_size as u32 + self.line_spacing;
            if h == 0 {
                12
            } else {
                (self.video_height / h).max(1)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = RenderOptions::default();
        assert_eq!(opts.video_width, 1920);
        assert_eq!(opts.video_height, 1080);
        assert_eq!(opts.font_name, "SimHei");
        assert_eq!(opts.font_size, 25);
        assert!((opts.opacity - 0.8).abs() < 0.01);
        assert_eq!(opts.display_duration, 8.0);
        assert!(opts.show_scroll);
        assert!(opts.show_top);
        assert!(opts.show_bottom);
    }

    #[test]
    fn test_effective_max_rows_1080p() {
        let opts = RenderOptions::default();
        // 1080 / (25 + 4) = 1080 / 29 ≈ 37
        let rows = opts.effective_max_rows();
        assert!(rows >= 36);
        assert!(rows <= 38);
    }

    #[test]
    fn test_effective_max_rows_720p() {
        let mut opts = RenderOptions::default();
        opts.video_height = 720;
        // 720 / 29 ≈ 24
        let rows = opts.effective_max_rows();
        assert!(rows >= 24);
        assert!(rows <= 25);
    }

    #[test]
    fn test_effective_max_rows_custom() {
        let mut opts = RenderOptions::default();
        opts.max_rows = Some(5);
        assert_eq!(opts.effective_max_rows(), 5);
    }

    #[test]
    fn test_effective_max_rows_zero_height() {
        let mut opts = RenderOptions::default();
        opts.font_size = 0;
        opts.line_spacing = 0;
        assert_eq!(opts.effective_max_rows(), 12);
    }
}
