//! 下载任务管理器.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};

use crate::{
    aria2c::Aria2cClient,
    event::DownloadEvent,
    job::JobState,
    parallel::{DownloadSpec, ParallelDownloader},
    range::RangeClient,
};

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
    /// 任务状态.
    job_state: Arc<Mutex<JobState>>,
    /// 取消标志 (AtomicBool, 可跨线程安全访问).
    cancelled: Arc<AtomicBool>,
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
            job_state: Arc::new(Mutex::new(JobState::Pending)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 获取当前任务状态.
    pub fn job_state(&self) -> JobState {
        self.job_state.lock().unwrap().clone()
    }

    /// 获取取消标志 (可用于传递给下载任务以支持协作式取消).
    pub fn cancelled_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    /// 取消下载任务.
    ///
    /// 设置取消标志并尝试将状态转换为 Cancelled.
    /// 终态 (Done/Failed/Cancelled) 不可取消, 返回 current state.
    pub fn cancel(&self) -> JobState {
        self.cancelled.store(true, Ordering::SeqCst);
        let mut state = self.job_state.lock().unwrap();
        match state.transition(&JobState::Cancelled) {
            Ok(new_state) => {
                *state = new_state.clone();
                new_state
            }
            Err(_) => state.clone(),
        }
    }

    /// 暂停下载任务.
    ///
    /// 只有 Running 状态可暂停.
    /// 返回转换后的状态 (Paused 或 current state).
    pub fn pause(&self) -> JobState {
        let mut state = self.job_state.lock().unwrap();
        match state.transition(&JobState::Paused) {
            Ok(new_state) => {
                *state = new_state.clone();
                new_state
            }
            Err(_) => state.clone(),
        }
    }

    /// 恢复下载任务.
    ///
    /// 只有 Paused 状态可恢复.
    /// 返回转换后的状态 (Running 或 current state).
    pub fn resume(&self) -> JobState {
        self.cancelled.store(false, Ordering::SeqCst);
        let mut state = self.job_state.lock().unwrap();
        match state.transition(&JobState::Running) {
            Ok(new_state) => {
                *state = new_state.clone();
                new_state
            }
            Err(_) => state.clone(),
        }
    }

    /// 标记任务为运行中.
    pub fn mark_running(&self) -> JobState {
        let mut state = self.job_state.lock().unwrap();
        match state.transition(&JobState::Running) {
            Ok(new_state) => {
                *state = new_state.clone();
                new_state
            }
            Err(_) => state.clone(),
        }
    }

    /// 标记任务为完成.
    pub fn mark_done(&self) -> JobState {
        let mut state = self.job_state.lock().unwrap();
        match state.transition(&JobState::Done) {
            Ok(new_state) => {
                *state = new_state.clone();
                new_state
            }
            Err(_) => state.clone(),
        }
    }

    /// 标记任务失败.
    pub fn mark_failed(&self, reason: impl Into<String>) -> JobState {
        let mut state = self.job_state.lock().unwrap();
        match state.transition(&JobState::Failed(reason.into())) {
            Ok(new_state) => {
                *state = new_state.clone();
                new_state
            }
            Err(_) => state.clone(),
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

        self.mark_running();

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
                self.mark_failed("至少需要视频或音频下载任务之一");
                return Err(anyhow!("至少需要视频或音频下载任务之一"));
            }
        };

        if let Some(spec) = subtitle {
            let mut subtitle_cb = on_event;
            let _ = self.parallel.download(spec, move |event| subtitle_cb(event)).await?;
        }

        self.mark_done();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_pending() {
        let mgr = DownloadManager::new(DownloadMode::Parallel);
        assert_eq!(mgr.job_state(), JobState::Pending);
    }

    #[test]
    fn test_mark_running() {
        let mgr = DownloadManager::new(DownloadMode::Parallel);
        let state = mgr.mark_running();
        assert_eq!(state, JobState::Running);
        assert_eq!(mgr.job_state(), JobState::Running);
    }

    #[test]
    fn test_pause_from_running() {
        let mgr = DownloadManager::new(DownloadMode::Parallel);
        mgr.mark_running();
        let state = mgr.pause();
        assert_eq!(state, JobState::Paused);
    }

    #[test]
    fn test_pause_fails_from_pending() {
        let mgr = DownloadManager::new(DownloadMode::Parallel);
        let state = mgr.pause();
        assert_eq!(state, JobState::Pending); // stays pending
    }

    #[test]
    fn test_cancel_from_pending() {
        let mgr = DownloadManager::new(DownloadMode::Parallel);
        let state = mgr.cancel();
        assert_eq!(state, JobState::Cancelled);
        assert!(mgr.cancelled_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn test_resume_from_paused() {
        let mgr = DownloadManager::new(DownloadMode::Parallel);
        mgr.mark_running();
        mgr.pause();
        let state = mgr.resume();
        assert_eq!(state, JobState::Running);
    }

    #[test]
    fn test_cancel_is_terminal() {
        let mgr = DownloadManager::new(DownloadMode::Parallel);
        mgr.cancel();
        // Cannot transition from Cancelled
        let state = mgr.mark_running();
        assert_eq!(state, JobState::Cancelled);
    }

    #[test]
    fn test_mark_failed() {
        let mgr = DownloadManager::new(DownloadMode::Parallel);
        mgr.mark_running();
        let state = mgr.mark_failed("network timeout");
        assert_eq!(state, JobState::Failed("network timeout".into()));
    }
}
