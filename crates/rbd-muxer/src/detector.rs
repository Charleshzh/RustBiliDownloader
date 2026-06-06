//! detector

use anyhow::Result;
use std::path::Path;

/// `MediaInfo`
#[derive(Debug, Clone, Default)]
pub struct MediaInfo {
    /// duration
    pub duration: f64,
    /// vcodec
    pub vcodec: String,
    /// width
    pub width: u32,
    /// height
    pub height: u32,
    /// acodec
    pub acodec: String,
}

/// probe
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
