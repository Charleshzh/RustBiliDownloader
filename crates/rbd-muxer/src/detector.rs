//! 媒体信息探测 (时长/编码/分辨率).

use anyhow::Result;
use std::path::Path;

/// 媒体文件信息 (时长/编码/分辨率).
#[derive(Debug, Clone, Default)]
pub struct MediaInfo {
    /// 时长 (秒).
    pub duration: f64,
    /// 视频编码器.
    pub vcodec: String,
    /// 宽度 (像素).
    pub width: u32,
    /// 高度 (像素).
    pub height: u32,
    /// 音频编码器.
    pub acodec: String,
}

/// 探测媒体文件信息.
pub fn probe(_path: &Path) -> Result<MediaInfo> {
    // Dummy implementation
    Ok(MediaInfo {
        duration: 0.0,
        vcodec: "avc".to_string(),
        width: 1920,
        height: 1080,
        acodec: "aac".to_string(),
    })
}
