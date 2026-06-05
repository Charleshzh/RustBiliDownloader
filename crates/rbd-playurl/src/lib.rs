//! # rbd-playurl
//!
//! playurl 抓取层. 4-mode WEB + 1-mode APP gRPC (备选 8K).
//!
//! **算法来源**:
//! - HTML5 / Web / WebBangumi / TV: 4 套 B 站 WEB 端点 (BBDown/Parser.cs + Yutto api/ugc_video.py)
//! - `FNVAL_DASH_ALL`: 单一参数同时解锁 dash+hdr+4k+dolby+8k+av1 (Yutto 2.x 协议)
//! - APP gRPC: `https://grpc.biliapi.net/bilibili.app.playurl.v1.PlayURL/PlayView` 备选路径 (BBDown AppHelper.cs, M4 启用)

#![warn(missing_docs)]

/// HTML5 playurl 端点 (最低优先级, 480P).
pub mod html5;
/// Web playurl 端点 (主用, `FNVAL_DASH_ALL` 拿高质量).
pub mod web;
/// Web 番剧 playurl 端点 (/pgc/view/web/season + /pgc/player/web/playurl).
pub mod web_bangumi;
/// TV playurl 端点 (/tv 路径, 备选 8K).
pub mod tv;
/// APP gRPC playurl (PlayView, M4 启用).
pub mod app_grpc;
/// playurl 统一客户端.
pub mod client;
/// 4 套端点 fallback 链调度.
pub mod fallback;

pub use client::PlayUrlClient;
pub use fallback::{FallbackChain, FetchResult, Mode};
pub use rbd_core::PlayUrlResponse;
