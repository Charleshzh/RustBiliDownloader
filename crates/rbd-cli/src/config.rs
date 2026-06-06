//! CLI 配置文件加载.
//!
//! 从 `~/.config/rbd/rbd.toml` 加载 CLI 配置,
//! 使用 `rbd_foundation::config` 提供的通用 TOML 加载器.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// CLI 配置.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct CliConfig {
    /// 默认画质
    pub quality: Option<u32>,
    /// 多线程数
    pub num_workers: Option<u32>,
    /// aria2c RPC 地址
    pub aria2c_rpc: Option<String>,
    /// 默认下载目录
    pub download_dir: Option<std::path::PathBuf>,
    /// 文件名模板
    pub file_pattern: Option<String>,
    /// 弹幕默认开启
    pub danmaku: bool,
    /// 字幕默认开启
    pub subtitle: bool,
    /// 封面默认开启
    pub cover: bool,
    /// 交互式选 track
    pub interactive: bool,
}

/// 加载配置文件.
///
/// 传入 `None` 则使用默认路径 `~/.config/rbd/rbd.toml`.
pub fn load(path: Option<&Path>) -> Result<CliConfig> {
    let config_path = path.map_or_else(rbd_foundation::config::config_path, Path::to_path_buf);
    if !config_path.exists() {
        return Ok(CliConfig::default());
    }
    rbd_foundation::config::load(&config_path)
}
