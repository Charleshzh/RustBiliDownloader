//! ѡ.

use std::fmt;

/// Mux .
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuxStrategy {
    /// Pure Rust, no ffmpeg, fastest.
    DashCopy,
    /// ffmpeg -c copy (compatible container remux).
    FfmpegMerge,
    /// ffmpeg full transcode (last resort).
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

/// ѡ Mux .
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
        "hevc" => MuxStrategy::FfmpegMerge,
        "av1" | "dvh1" | "dvhe" => MuxStrategy::FfmpegMerge,
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
        assert_eq!(
            MuxStrategy::FfmpegTranscode.to_string(),
            "FfmpegTranscode"
        );
    }
}
