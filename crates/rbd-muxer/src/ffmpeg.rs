//! ffmpeg wrapper

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `FfmpegMuxer`
#[derive(Debug, Clone)]
pub struct FfmpegMuxer {
    ffmpeg_path: PathBuf,
}

impl FfmpegMuxer {
    /// new
    pub fn new() -> Result<Self> {
        // Just a dummy locator
        Ok(Self {
            ffmpeg_path: PathBuf::from("ffmpeg"),
        })
    }

    /// `with_path`
    #[must_use]
    pub fn with_path(path: PathBuf) -> Self {
        Self { ffmpeg_path: path }
    }

    /// `merge_copy`
    pub fn merge_copy(&self, video: &Path, audio: Option<&Path>, output: &Path) -> Result<()> {
        let mut cmd = Command::new(&self.ffmpeg_path);
        cmd.arg("-i").arg(video);
        if let Some(a) = audio {
            cmd.arg("-i").arg(a);
        }
        cmd.arg("-c").arg("copy").arg(output);

        let status = cmd
            .status()
            .map_err(|e| anyhow::anyhow!("ffmpeg 启动失败: {e}"))?;
        if !status.success() {
            anyhow::bail!(
                "ffmpeg 合流失败 (exit code {})",
                status.code().unwrap_or(-1)
            );
        }
        Ok(())
    }

    /// transcode
    pub fn transcode(
        &self,
        video: &Path,
        audio: Option<&Path>,
        output: &Path,
        target_vcodec: &str,
        target_acodec: &str,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.ffmpeg_path);
        cmd.arg("-i").arg(video);
        if let Some(a) = audio {
            cmd.arg("-i").arg(a);
        }
        cmd.arg("-c:v")
            .arg(target_vcodec)
            .arg("-c:a")
            .arg(target_acodec)
            .arg(output);

        let status = cmd
            .status()
            .map_err(|e| anyhow::anyhow!("ffmpeg 启动失败: {e}"))?;
        if !status.success() {
            anyhow::bail!(
                "ffmpeg 转码失败 (exit code {})",
                status.code().unwrap_or(-1)
            );
        }
        Ok(())
    }
}
