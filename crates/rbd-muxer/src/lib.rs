//! # rbd-muxer
//!
//! 混流层: DASH copy (纯 Rust) + ffmpeg-sidecar 兜底 (杜比视界/HDR/转码).
//!
//! **算法来源**:
//! - BBDown/BBDownMuxer.cs: 3 种混流路径 (DASH m4s / FLV / 杜比视界)
//! - Yutto/utils/ffmpeg.py: `FFmpegCommandBuilder` 链式 API (本 crate 用 enum state machine 替代)
//! - mp4box: 杜比视界回退 (用户决策 Q7: ffmpeg 5.0+ 优先, 不需要 mp4box)
//!
//! **决策** (用户 Q6+Q7):
//! - Q6: 仅 DASH copy 纯 Rust, codec 转码走 ffmpeg
//! - Q7: 杜比视界走 ffmpeg-sidecar (要求 ffmpeg 5.0+)

#![warn(missing_docs)]

/// ffmpeg 命令构造.
pub mod command;
/// DASH m4s 容器混流 (纯 Rust, 不调 ffmpeg).
pub mod dash_copy;
/// 媒体信息探测 (时长/编码/分辨率).
pub mod detector;
/// ffmpeg-sidecar 命令包装.
pub mod ffmpeg;
/// MP4 metadata 写入 (moov box).
pub mod metadata;
/// 混流策略选择 (DASH-copy / ffmpeg-merge / ffmpeg-transcode).
pub mod strategy;

pub use dash_copy::DashCopyMuxer;
pub use ffmpeg::FfmpegMuxer;
pub use metadata::write_metadata;
pub use strategy::{choose_strategy, MuxStrategy};
