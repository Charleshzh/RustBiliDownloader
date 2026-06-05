//! 下载事件.

use std::path::PathBuf;

/// 下载过程事件.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// 开始下载.
    Start {
        /// 任务 ID.
        task_id: String,
        /// 总大小.
        total: u64,
    },
    /// 进度更新.
    Progress {
        /// 任务 ID.
        task_id: String,
        /// 已下载字节数.
        downloaded: u64,
        /// 总大小.
        total: u64,
        /// 当前速度.
        speed_bps: f64,
    },
    /// 下载完成.
    Done {
        /// 任务 ID.
        task_id: String,
        /// 输出路径.
        path: PathBuf,
    },
    /// 下载失败.
    Failed {
        /// 任务 ID.
        task_id: String,
        /// 错误信息.
        error: String,
    },
    /// 日志.
    Log {
        /// 任务 ID.
        task_id: String,
        /// 日志消息.
        message: String,
    },
}
