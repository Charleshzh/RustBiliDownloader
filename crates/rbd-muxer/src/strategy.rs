//! 混流策略选择.

use std::fmt;

/// 混流策略枚举.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuxStrategy {
    /// 纯 Rust DASH 复制, 不依赖 ffmpeg, 最快.
    DashCopy,
    /// ffmpeg 流复制 (-c copy, 兼容容器重封装).
    FfmpegMerge,
    /// ffmpeg 完整转码 (兜底方案).
    FfmpegTranscode,
}

impl fmt::Display for MuxStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DashCopy => write!(f, "DashCopy"),
            Self::FfmpegMerge => write!(f, "FfmpegMerge"),
            Self::FfmpegTranscode => write!(f, "FfmpegTranscode"),
        }
    }
}

/// 根据编码类型和特性选择混流策略.
#[must_use]
pub fn choose_strategy(
    video_codec: &str,
    _audio_codec: &str,
    _is_hdr: bool,
    is_dolby_vision: bool,
) -> MuxStrategy {
    if is_dolby_vision {
        return MuxStrategy::FfmpegTranscode;
    }
    match video_codec {
        "hevc" | "av1" | "dvh1" | "dvhe" => MuxStrategy::FfmpegMerge,
        _ => MuxStrategy::DashCopy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_strategy_avc_aac() {
        assert_eq!(
            choose_strategy("avc", "aac", false, false),
            MuxStrategy::DashCopy
        );
    }

    #[test]
    fn test_choose_strategy_hevc() {
        assert_eq!(
            choose_strategy("hevc", "aac", false, false),
            MuxStrategy::FfmpegMerge
        );
    }

    #[test]
    fn test_choose_strategy_dolby_vision() {
        assert_eq!(
            choose_strategy("hevc", "aac", true, true),
            MuxStrategy::FfmpegTranscode
        );
    }

    #[test]
    fn test_strategy_name() {
        assert_eq!(MuxStrategy::DashCopy.to_string(), "DashCopy");
        assert_eq!(MuxStrategy::FfmpegMerge.to_string(), "FfmpegMerge");
        assert_eq!(MuxStrategy::FfmpegTranscode.to_string(), "FfmpegTranscode");
    }
}
