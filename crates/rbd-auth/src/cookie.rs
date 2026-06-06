//! Cookie 字符串解析 — Set-Cookie / curl 格式互转.
//!
//! **算法来源**: BBDown `BBDownLoginUtil` cookie 解析逻辑.

use std::collections::HashMap;

/// 解析 `"key=value; key=value; ..."` 格式的 cookie 字符串.
///
/// 示例: `"SESSDATA=abc; bili_jct=def"` → `{"SESSDATA": "abc", "bili_jct": "def"}`
#[must_use]
pub fn parse_cookie_string(s: &str) -> HashMap<String, String> {
    s.split(';')
        .filter_map(|part| {
            let part = part.trim();
            part.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

/// 从 HTTP Set-Cookie 头部列表提取 cookie 键值对.
///
/// 每个头部形如: `"SESSDATA=abc; Path=/; Domain=.bilibili.com; HttpOnly"`
#[must_use]
pub fn parse_set_cookie(headers: &[String]) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    for header in headers {
        // Set-Cookie header: first semicolon-separated segment is the key=value
        if let Some(pair) = header.split(';').next() {
            let pair = pair.trim();
            if let Some((key, value)) = pair.split_once('=') {
                cookies.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }
    cookies
}

/// 将 cookie map 转为 `curl` 风格的 Cookie 头.
///
/// 示例: `{"SESSDATA": "abc"}` → `"SESSDATA=abc"`
#[must_use]
pub fn to_cookie_header<S: std::hash::BuildHasher>(cookies: &HashMap<String, String, S>) -> String {
    cookies
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// 合并两个 cookie map, `other` 中的键会覆盖 `base`.
pub fn merge_cookies<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(base: &mut HashMap<String, String, S1>, other: &HashMap<String, String, S2>) {
    for (k, v) in other {
        base.insert(k.clone(), v.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cookie_string_basic() {
        let map = parse_cookie_string("SESSDATA=abc; bili_jct=def");
        assert_eq!(map.get("SESSDATA").map(String::as_str), Some("abc"));
        assert_eq!(map.get("bili_jct").map(String::as_str), Some("def"));
    }

    #[test]
    fn test_parse_cookie_string_handles_spaces() {
        let map = parse_cookie_string(" SESSDATA = abc ; bili_jct = def ");
        assert_eq!(map.get("SESSDATA").map(String::as_str), Some("abc"));
    }

    #[test]
    fn test_parse_set_cookie_single() {
        let headers = &["SESSDATA=abc; Path=/; Domain=.bilibili.com".to_string()];
        let map = parse_set_cookie(headers);
        assert_eq!(map.get("SESSDATA").map(String::as_str), Some("abc"));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_parse_set_cookie_multiple() {
        let headers = &[
            "buvid3=abc123; Path=/; Domain=.bilibili.com".to_string(),
            "buvid4=def456; Path=/".to_string(),
        ];
        let map = parse_set_cookie(headers);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("buvid3").map(String::as_str), Some("abc123"));
        assert_eq!(map.get("buvid4").map(String::as_str), Some("def456"));
    }

    #[test]
    fn test_to_cookie_header() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "v1".to_string());
        map.insert("key2".to_string(), "v2".to_string());
        let header = to_cookie_header(&map);
        assert!(header.contains("key1=v1"));
        assert!(header.contains("key2=v2"));
    }

    #[test]
    fn test_merge_cookies_b_overrides_a() {
        let mut base = HashMap::from([
            ("a".to_string(), "old".to_string()),
            ("shared".to_string(), "base".to_string()),
        ]);
        let other = HashMap::from([
            ("b".to_string(), "new".to_string()),
            ("shared".to_string(), "override".to_string()),
        ]);
        merge_cookies(&mut base, &other);
        assert_eq!(base.get("a").map(String::as_str), Some("old"));
        assert_eq!(base.get("b").map(String::as_str), Some("new"));
        assert_eq!(base.get("shared").map(String::as_str), Some("override"));
    }

    #[test]
    fn test_parse_set_cookie_empty() {
        let map = parse_set_cookie(&[]);
        assert!(map.is_empty());
    }

    #[test]
    fn test_to_cookie_header_empty() {
        let map = HashMap::new();
        assert_eq!(to_cookie_header(&map), "");
    }
}
