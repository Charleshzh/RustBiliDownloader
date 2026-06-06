//! ffmpeg 包装器.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `FfmpegMuxer`
#[derive(Debug, Clone)]
pub struct FfmpegMuxer {
    ffmpeg_path: PathBuf,
}

impl FfmpegMuxer {
    /// 创建 ffmpeg 混流器 (自动查找 ffmpeg 路径).
    pub fn new() -> Result<Self> {
        // Just a dummy locator
        Ok(Self {
            ffmpeg_path: PathBuf::from("ffmpeg"),
        })
    }

    /// 指定 ffmpeg 二进制路径.
    #[must_use]
    pub fn with_path(path: PathBuf) -> Self {
        Self { ffmpeg_path: path }
    }

    /// 复制流合并 (不重新编码, 最快).
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

    /// 完整转码 (需要重新编码, 兜底方案).
    pub fn transcode(
        &self,
        video: &Path,
        audio: Option<&Path>,
        output: &Path,
        target_vcodec: &str,
        output_acodec: &str,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.ffmpeg_path);
        cmd.arg("-i").arg(video);
        if let Some(a) = audio {
            cmd.arg("-i").arg(a);
        }
        cmd.arg("-c:v")
            .arg(target_vcodec)
            .arg("-c:a")
            .arg(output_acodec)
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
