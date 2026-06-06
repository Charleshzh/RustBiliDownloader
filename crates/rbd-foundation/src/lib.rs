//! # rbd-foundation
//!
//! RBD 基础原语: 错误 / 日志 / 路径 / 进度 / 配置 / 限流.
//! 这是所有其他 crate 共享的底层设施, 自身不依赖任何 rbd-* crate.

#![warn(missing_docs)]

/// 通用编解码 (hex/Base58/URL/MD5/SHA1).
pub mod codec;
/// TOML 配置加载/保存.
pub mod config;
/// 错误类型与 Result 别名.
pub mod error;
/// i18n 占位 (v1.0 仅中文).
pub mod locale;
/// 日志初始化 (stderr/file/both).
pub mod log;
/// 路径处理 (sanitize/unique/temp).
pub mod path;
/// 进度条.
pub mod progress;
/// 限流 (基于 governor).
pub mod ratelimit;
/// 指数退避重试.
pub mod retry;
/// 文件名模板 (tera).
pub mod template;
/// 版本常量与打印.
pub mod version;
