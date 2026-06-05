//! # rbd-subtitle
//!
//! 字幕层: 5 套 API 回退链 + JSON/SRT/ASS 互转.
//!
//! **算法来源**: BBDown/SubUtil.cs 5 套 API 顺序 + Yutto/api/ugc_video.py `get_ugc_video_subtitles` + WBI 签名 (M2 集成).
//!
//! **API 顺序 (回退链)**:
//! 1. `GET /x/player/wbi/v2?cid=xxx&bvid=xxx` (主用, 需 WBI 签名)
//! 2. `GET /x/player/v2?cid=xxx&bvid=xxx` (旧 fallback)
//! 3. `GET /x/player.so?id=cid` (JSON 格式) — TODO M5
//! 4. `GET /x/web-interface/view?bvid=xxx` (视频元信息里的 subtitle 字段) — TODO M5
//! 5. `GET /x/v2/dm/view?type=1&oid=cid` (含字幕元数据) — TODO M5

#![warn(missing_docs)]

/// 字幕抓取 (5 套 API 回退链).
pub mod fetch;
/// 字幕格式转换 (JSON ↔ SRT ↔ ASS).
pub mod convert;
/// 字幕格式探测.
pub mod format;
/// 字幕数据模型.
pub mod model;
/// 字幕 fallback 链调度.
pub mod fallback;

pub use model::{Subtitle, SubtitleBody, SubtitleEntry, SubtitleFormat};
pub use format::{detect_from_content, detect_from_url};
pub use convert::{ass_to_srt, json_to_srt, srt_to_ass};
pub use fetch::{fetch_subtitle_content, fetch_subtitle_list, parse_subtitle_list};
pub use fallback::SubtitleFallback;
