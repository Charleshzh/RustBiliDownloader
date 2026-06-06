//! metadata

use anyhow::Result;
use std::path::Path;

/// `write_metadata`
pub async fn write_metadata(
    _output: &Path,
    _title: &str,
    _artist: &str,
    _cover: Option<&Path>,
) -> Result<()> {
    // Dummy implementation
    Ok(())
}
