//! # rbd-danmaku
//!
//! 弹幕层: 3 种输入 (XML / Web protobuf / Protobuf 二进制) → 1 种输出 (ASS).
//!
//! **算法来源**: biliass-rust 公开算法 (yutto-dev/biliass), 但 **完全重写** (避开 GPL-3.0 污染).
//! - 颜色: `alpha = 255 - (opacity * 255)`, 转为 `\c&H{BBGGRR}&` 格式
//! - 字号: `font_size` 范围 12-36
//! - 滚动/顶部/底部/逆向/高级动画 (旋转/平移/缩放) 全部支持
//! - 渲染顺序: 按 (timeline, timestamp, no, content, pos, color, size) 排序, 用 rustc-hash 快排
//! - 并行: `rayon::into_par_iter` 处理多视频同时转码

#![warn(missing_docs)]

/// 颜色计算 (alpha + BGR 转换).
pub mod color;
/// 布局算法 (row conflict resolution).
pub mod layout;
/// 弹幕数据模型.
pub mod model;
/// 弹幕渲染选项.
pub mod options;
/// 弹幕读取 (XML / protobuf / web protobuf).
pub mod reader;
/// 弹幕渲染 (XML/protobuf → ASS 转换).
pub mod render;
/// 弹幕写入 (ASS / JSON 调试).
pub mod writer;

pub use color::{ass_color_alpha_tag, opacity_to_ass_alpha, outline_width, rgb_to_ass_color};
pub use layout::RowTracker;
pub use model::{Danmaku, DanmakuList, DanmakuMode};
pub use options::RenderOptions;
pub use reader::{parse_json, parse_xml, protobuf};
pub use render::{render_batch_parallel, render_to_ass};
pub use writer::{write_danmaku, write_head};
