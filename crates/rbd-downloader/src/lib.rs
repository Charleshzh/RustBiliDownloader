//! # rbd-downloader
//!
//! 下载器: 多线程 HTTP Range 分片 + 协程并发 video+audio + aria2c JSON-RPC.
//!
//! **算法来源**:
//! - 多线程 Range: BBDown/BBDownDownloadUtil.cs `MultiThreadDownloadFileAsync`
//! - 协程并发 video+audio: Yutto/downloader.py `asyncio.gather(video, audio, progress)` — BBDown 缺失!
//! - aria2c JSON-RPC: BBDown/BBDownAria2c.cs JSON-RPC over HTTP (默认 6800 端口)
//! - 进度回调: 64KB chunks, `tokio::sync::mpsc::Sender<DownloadEvent>`

#![warn(missing_docs)]

/// HTTP Range 客户端 (单连接多线程分片).
pub mod range;
/// 多线程并行下载器.
pub mod parallel;
/// aria2c JSON-RPC 客户端.
pub mod aria2c;
/// 进度条 (基于 indicatif).
pub mod progress;
/// 下载任务管理器.
pub mod manager;
/// 下载事件 (进度/完成/失败).
pub mod event;

pub use aria2c::{Aria2Status, Aria2cClient};
pub use event::DownloadEvent;
pub use manager::{DownloadManager, DownloadMode};
pub use parallel::{calculate_blocks, DownloadSpec, ParallelDownloader};
pub use progress::DownloadProgress;
pub use range::{RangeClient, RangeResponse};
