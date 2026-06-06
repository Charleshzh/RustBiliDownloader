//! 路径处理: 跨平台临时目录, 文件名清理, URL 转安全文件名.

use std::path::{Path, PathBuf};

/// 跨平台临时目录 (RBD 专属子目录).
///
/// 优先级: `$RBD_TEMP_DIR` > `std::env::temp_dir()/rbd-XXX` (`XXX` 为 nano timestamp).
#[must_use]
pub fn rbd_temp_dir() -> PathBuf {
    if let Ok(p) = std::env::var("RBD_TEMP_DIR") {
        return PathBuf::from(p);
    }
    let mut dir = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.push(format!("rbd-{nanos}"));
    dir
}

/// 创建临时目录, 返回 [`PathBuf`].
pub fn create_temp_dir(prefix: &str) -> std::io::Result<PathBuf> {
    let dir = rbd_temp_dir().join(prefix);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 清理非法文件名字符 (Windows 严格, 跨平台宽松).
///
/// 替换规则: `\/:*?"<>|` → ` ` (全角空格).
#[must_use]
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '\u{3000}',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .to_string()
}

/// URL 转安全文件名 (用于下载默认文件名).
#[must_use]
pub fn url_to_filename(url: &str) -> Option<String> {
    let url = url.split('?').next()?;
    let path = url.split('?').next()?.trim_end_matches('/');
    let last = path.rsplit('/').next()?;
    if last.is_empty() {
        None
    } else {
        Some(sanitize_filename(last))
    }
}

/// 拼装输出路径.
///
/// `base` = 下载根目录, `relative` = 相对路径 (如 `<title>/<p_title>.mp4`).
#[must_use]
pub fn join_output(base: &Path, relative: &str) -> PathBuf {
    let sanitized: PathBuf = relative.split('/').map(sanitize_filename).collect();
    base.join(sanitized)
}

/// 重复文件重命名: `foo.mp4` → `foo (1).mp4` → `foo (2).mp4`.
#[must_use]
pub fn unique_path(target: &Path) -> PathBuf {
    if !target.exists() {
        return target.to_path_buf();
    }
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = target.extension().and_then(|s| s.to_str()).unwrap_or("");
    for n in 1..=9999 {
        let candidate = if ext.is_empty() {
            parent.join(format!("{stem} ({n})"))
        } else {
            parent.join(format!("{stem} ({n}).{ext}"))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    target.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("abc/def:1"), "abc\u{3000}def\u{3000}1");
        assert_eq!(sanitize_filename("..foo."), "..foo");
    }

    #[test]
    fn test_url_to_filename() {
        assert_eq!(
            url_to_filename("https://x.com/a/b.flv"),
            Some("b.flv".into())
        );
        assert_eq!(
            url_to_filename("https://x.com/a/b.flv?token=1"),
            Some("b.flv".into())
        );
    }

    #[test]
    fn test_unique_path() {
        let tmp = tempfile::tempdir().unwrap();
        let p1 = unique_path(&tmp.path().join("foo.mp4"));
        std::fs::write(&p1, b"x").unwrap();
        let p2 = unique_path(&tmp.path().join("foo.mp4"));
        assert!(p2.to_str().unwrap().contains("(1)"));
    }
}
