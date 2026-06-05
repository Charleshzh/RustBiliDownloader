//! dash_copy — DASH m4s → MP4 混流.
//!
//! **v1.0 策略**: 委托 ffmpeg 执行 DASH remux, 因为简单的字节拼接会产生损坏的 MP4.
//! 后续版本可引入 `mp4` crate 实现纯 Rust m4s box 重组.

use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};

/// DashCopyMuxer — DASH 片段混流器.
///
/// 目前委托 `FfmpegMuxer::merge_copy` 完成.
#[derive(Debug, Clone, Default)]
pub struct DashCopyMuxer;

impl DashCopyMuxer {
    /// 创建混流器.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 将 DASH m4s 片段混流为 MP4.
    ///
    /// 使用 ffmpeg 进行正确的 fragment MP4 remux,
    /// 而不是简单的字节拼接 (会产生损坏的 MP4 文件).
    pub fn mux(
        &self,
        video_m4s_files: &[PathBuf],
        audio_m4s_files: Option<&[PathBuf]>,
        output: &Path,
    ) -> Result<()> {
        let muxer = super::ffmpeg::FfmpegMuxer::new()?;
        let video = video_m4s_files
            .first()
            .ok_or_else(|| anyhow!("缺少视频 m4s 文件"))?;
        let audio = audio_m4s_files.and_then(|a| a.first());
        muxer.merge_copy(video, audio.map(|a| a.as_path()), output)
    }
}
