//! 下载任务管理器.

use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::{aria2c::Aria2cClient, event::DownloadEvent, parallel::{DownloadSpec, ParallelDownloader}, range::RangeClient};

/// 下载模式.
#[derive(Debug, Clone)]
pub enum DownloadMode {
    /// 内建并行下载.
    Parallel,
    /// aria2c 外部下载.
    Aria2c,
}

/// 下载管理器.
pub struct DownloadManager {
    parallel: ParallelDownloader,
    aria2c: Option<Aria2cClient>,
    video_tasks: Vec<String>,
    audio_tasks: Vec<String>,
}

impl DownloadManager {
    /// 创建管理器.
    #[must_use]
    pub fn new(mode: DownloadMode) -> Self {
        let parallel = ParallelDownloader::new(RangeClient::new());
        let aria2c = match mode {
            DownloadMode::Parallel => None,
            DownloadMode::Aria2c => Some(Aria2cClient::default()),
        };
        Self {
            parallel,
            aria2c,
            video_tasks: Vec::new(),
            audio_tasks: Vec::new(),
        }
    }

    /// 并发下载视频/音频/字幕.
    ///
    /// 支持三种模式:
    /// - 视频+音频: 并行下载两者
    /// - 仅视频: 只下载视频
    /// - 仅音频: 只下载音频
    ///
    /// 返回 `(video_path, audio_path)`, 缺失的流对应 `None`.
    pub async fn download_concurrent(
        &self,
        video: Option<DownloadSpec>,
        audio: Option<DownloadSpec>,
        subtitle: Option<DownloadSpec>,
        on_event: impl FnMut(DownloadEvent) + Send + Clone,
    ) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
        if self.aria2c.is_some() {
            return Err(anyhow!("Aria2c 模式将在 M4 接入 download_concurrent"));
        }

        let video_future = if let Some(video_spec) = video {
            let mut cb = on_event.clone();
            Some(self.parallel.download(video_spec, move |event| cb(event)))
        } else {
            None
        };

        let audio_future = if let Some(audio_spec) = audio {
            let mut cb = on_event.clone();
            Some(self.parallel.download(audio_spec, move |event| cb(event)))
        } else {
            None
        };

        // 并发执行所有存在的下载任务
        let (video_path, audio_path) = match (video_future, audio_future) {
            (Some(vf), Some(af)) => {
                let (vp, ap) = tokio::join!(vf, af);
                (Some(vp?), Some(ap?))
            }
            (Some(vf), None) => {
                (Some(vf.await?), None)
            }
            (None, Some(af)) => {
                (None, Some(af.await?))
            }
            (None, None) => {
                return Err(anyhow!("至少需要视频或音频下载任务之一"));
            }
        };

        if let Some(spec) = subtitle {
            let mut subtitle_cb = on_event;
            let _ = self.parallel.download(spec, move |event| subtitle_cb(event)).await?;
        }

        Ok((video_path, audio_path))
    }

    /// 视频任务缓存.
    #[must_use]
    pub fn video_tasks(&self) -> &[String] {
        &self.video_tasks
    }

    /// 音频任务缓存.
    #[must_use]
    pub fn audio_tasks(&self) -> &[String] {
        &self.audio_tasks
    }
}
