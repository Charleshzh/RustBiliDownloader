//! CLI 进度条包装.
//!
//! 基于 `indicatif` 提供可视化下载进度.

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// CLI 进度条.
#[derive(Clone)]
pub struct CliProgress {
    bar: ProgressBar,
}

impl CliProgress {
    /// 创建进度条.
    #[must_use]
    pub fn new(total: u64, msg: &str) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_style(
            ProgressStyle::with_template(
                "{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} ETA {eta}",
            )
            .expect("valid progress style template"),
        );
        bar.set_message(msg.to_string());
        bar.enable_steady_tick(Duration::from_millis(100));
        Self { bar }
    }

    /// 增加已下载量.
    #[allow(dead_code)]
    pub fn inc(&self, delta: u64) {
        self.bar.inc(delta);
    }

    /// 设置当前进度.
    pub fn set_position(&self, pos: u64) {
        self.bar.set_position(pos);
    }

    /// 完成进度条.
    pub fn finish(&self) {
        self.bar.finish();
    }

    /// 完成并显示消息.
    pub fn finish_with_message(&self, msg: &str) {
        self.finish();
        self.bar.set_message(msg.to_string());
    }
}
