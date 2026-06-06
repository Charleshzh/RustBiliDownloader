//! MP4 元数据写入 (moov box).

use anyhow::Result;
use std::path::Path;

/// 写入 MP4 元数据 (标题/艺术家/封面).
pub async fn write_metadata(
    _output: &Path,
    _title: &str,
    _artist: &str,
    _cover: Option<&Path>,
) -> Result<()> {
    // Dummy implementation
    Ok(())
}
