//! TOML 配置文件加载/保存.
//!
//! 配置文件路径:
//! - 优先 `$RBD_CONFIG` 环境变量
//! - 否则 `~/.config/rbd/rbd.toml` (Linux/macOS)
//! - 否则 `%APPDATA%\rbd\rbd.toml` (Windows)

use std::path::{Path, PathBuf};

/// RBD 配置文件名.
pub const CONFIG_FILENAME: &str = "rbd.toml";

/// 定位配置文件路径 (不要求存在).
#[must_use]
pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("RBD_CONFIG") {
        return PathBuf::from(p);
    }
    if let Some(mut p) = dirs::config_dir() {
        p.push("rbd");
        p.push(CONFIG_FILENAME);
        return p;
    }
    PathBuf::from(CONFIG_FILENAME)
}

/// 加载 TOML 配置文件, 不存在则返回默认值.
pub fn load_or_default<T: serde::de::DeserializeOwned + Default>() -> T {
    load(&config_path()).unwrap_or_default()
}

/// 加载 TOML 配置文件.
pub fn load<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    if !path.exists() {
        anyhow::bail!("配置文件不存在: {}", path.display());
    }
    let text = std::fs::read_to_string(path)?;
    let value: T = toml::from_str(&text)?;
    Ok(value)
}

/// 保存 TOML 配置文件 (含目录创建).
pub fn save<T: serde::Serialize>(value: &T, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(value)?;
    std::fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
    struct TestCfg {
        #[serde(default)]
        name: String,
        #[serde(default)]
        count: u32,
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        let original = TestCfg {
            name: "rbd".to_string(),
            count: 42,
        };
        save(&original, &path).unwrap();
        let loaded: TestCfg = load(&path).unwrap();
        assert_eq!(original, loaded);
    }

    #[test]
    fn test_load_missing() {
        let path = PathBuf::from("/nonexistent/rbd.toml");
        let result: anyhow::Result<TestCfg> = load(&path);
        assert!(result.is_err());
    }
}
