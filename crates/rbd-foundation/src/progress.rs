//! 进度抽象. 单一进度条 (indivisible) 与多进度条 (multi-bar) 两种模式.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

/// 进度句柄 (克隆共享).
#[derive(Clone)]
pub struct Progress {
    inner: Arc<ProgressInner>,
}

enum ProgressInner {
    Single(ProgressBar),
    Multi(MultiProgress),
}

impl Progress {
    /// 创建单个进度条.
    #[must_use]
    pub fn single(len: u64, msg: &str) -> Self {
        let pb = ProgressBar::new(len);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {speed}")
                .unwrap()
                .progress_chars("##-"),
        );
        pb.set_message(msg.to_string());
        Self {
            inner: Arc::new(ProgressInner::Single(pb)),
        }
    }

    /// 创建多进度容器.
    #[must_use]
    pub fn multi() -> Self {
        Self {
            inner: Arc::new(ProgressInner::Multi(MultiProgress::new())),
        }
    }

    /// 添加子进度条.
    #[must_use]
    pub fn add(&self, len: u64, msg: &str) -> ProgressBar {
        match &*self.inner {
            ProgressInner::Single(pb) => pb.clone(),
            ProgressInner::Multi(mp) => {
                let pb = mp.add(ProgressBar::new(len));
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap()
                        .progress_chars("##-"),
                );
                pb.set_message(msg.to_string());
                pb
            }
        }
    }

    /// 全部结束.
    pub fn finish(&self) {
        match &*self.inner {
            ProgressInner::Single(pb) => pb.finish_and_clear(),
            ProgressInner::Multi(_) => {}
        }
    }
}
