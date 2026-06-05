//! 鉴权资料模型 — 多 profile SESSDATA 管理 + TOML 序列化.
//!
//! **算法来源**: Yutto auth.py `AuthFileModel.profiles` + BBDown `BBDownLoginUtil` cookie 解析.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 鉴权资料 (token / cookie / buvid).
///
/// 支持 TOML 序列化, 用于 keyring 持久化和配置文件导出.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AuthProfile {
    /// Profile 名 (用于 keyring lookup).
    pub name: String,
    /// 用户 mid (0 = 未登录).
    pub mid: u64,
    /// 用户名.
    pub uname: String,
    /// 主 cookie — SESSDATA.
    pub sessdata: String,
    /// CSRF token — bili_jct.
    pub bili_jct: String,
    /// TV 登录专用 — DedeUserID.
    pub dedeuserid: String,
    /// TV 登录专用 — access_token 有效期.
    pub ac_time_value: String,
    /// 设备指纹 v3.
    pub buvid3: String,
    /// 设备指纹 v4.
    pub buvid4: String,
    /// 其他未归类的 cookie 键值对.
    pub cookies: HashMap<String, String>,
}

impl AuthProfile {
    /// 是否有效登录态 (至少提供了 SESSDATA).
    #[must_use]
    pub fn is_logged_in(&self) -> bool {
        !self.sessdata.is_empty()
    }

    /// 生成 `Cookie:` 请求头 (curl 风格).
    ///
    /// 格式: `SESSDATA=xxx; bili_jct=yyy; DedeUserID=zzz; ...`
    #[must_use]
    pub fn cookie_header(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if !self.sessdata.is_empty() {
            parts.push(format!("SESSDATA={}", self.sessdata));
        }
        if !self.bili_jct.is_empty() {
            parts.push(format!("bili_jct={}", self.bili_jct));
        }
        if !self.dedeuserid.is_empty() {
            parts.push(format!("DedeUserID={}", self.dedeuserid));
        }
        if !self.ac_time_value.is_empty() {
            parts.push(format!("ac_time_value={}", self.ac_time_value));
        }
        if !self.buvid3.is_empty() {
            parts.push(format!("buvid3={}", self.buvid3));
        }
        if !self.buvid4.is_empty() {
            parts.push(format!("buvid4={}", self.buvid4));
        }

        for (key, value) in &self.cookies {
            parts.push(format!("{key}={value}"));
        }

        parts.join("; ")
    }

    /// 从 curl 风格的 Cookie 字符串解析.
    ///
    /// 示例输入: `"SESSDATA=abc; bili_jct=def; DedeUserID=123"`
    #[must_use]
    pub fn from_cookie_string(s: &str) -> Self {
        let mut profile = Self::default();
        let mut cookies = HashMap::new();

        for pair in s.split(';') {
            let pair = pair.trim();
            if let Some((key, value)) = pair.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "SESSDATA" => profile.sessdata = value.to_string(),
                    "bili_jct" => profile.bili_jct = value.to_string(),
                    "DedeUserID" => profile.dedeuserid = value.to_string(),
                    "ac_time_value" => profile.ac_time_value = value.to_string(),
                    "buvid3" => profile.buvid3 = value.to_string(),
                    "buvid4" => profile.buvid4 = value.to_string(),
                    _ => {
                        cookies.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }
        profile.cookies = cookies;
        profile
    }

    /// 合并另一个 profile 的字段 (后者优先).
    pub fn merge(&mut self, other: &Self) {
        if !other.sessdata.is_empty() {
            self.sessdata = other.sessdata.clone();
        }
        if !other.bili_jct.is_empty() {
            self.bili_jct = other.bili_jct.clone();
        }
        if !other.dedeuserid.is_empty() {
            self.dedeuserid = other.dedeuserid.clone();
        }
        if !other.buvid3.is_empty() {
            self.buvid3 = other.buvid3.clone();
        }
        if !other.buvid4.is_empty() {
            self.buvid4 = other.buvid4.clone();
        }
        if other.mid != 0 {
            self.mid = other.mid;
        }
        if !other.uname.is_empty() {
            self.uname = other.uname.clone();
        }
        for (k, v) in &other.cookies {
            self.cookies.insert(k.clone(), v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_logged_in_empty_sessdata() {
        let profile = AuthProfile::default();
        assert!(!profile.is_logged_in());
    }

    #[test]
    fn test_is_logged_in_with_sessdata() {
        let profile = AuthProfile {
            sessdata: "token123".to_string(),
            ..Default::default()
        };
        assert!(profile.is_logged_in());
    }

    #[test]
    fn test_cookie_header_format() {
        let profile = AuthProfile {
            sessdata: "abc".to_string(),
            bili_jct: "def".to_string(),
            ..Default::default()
        };
        let header = profile.cookie_header();
        assert!(header.contains("SESSDATA=abc"));
        assert!(header.contains("bili_jct=def"));
    }

    #[test]
    fn test_cookie_header_with_extra_cookies() {
        let mut cookies = HashMap::new();
        cookies.insert("sid".to_string(), "sid123".to_string());
        let profile = AuthProfile {
            sessdata: "abc".to_string(),
            bili_jct: "def".to_string(),
            cookies,
            ..Default::default()
        };
        let header = profile.cookie_header();
        assert!(header.contains("sid=sid123"));
    }

    #[test]
    fn test_from_cookie_string_parses_known_fields() {
        let s = "SESSDATA=abc; bili_jct=def; DedeUserID=123; buvid3=b3";
        let profile = AuthProfile::from_cookie_string(s);
        assert_eq!(profile.sessdata, "abc");
        assert_eq!(profile.bili_jct, "def");
        assert_eq!(profile.dedeuserid, "123");
        assert_eq!(profile.buvid3, "b3");
    }

    #[test]
    fn test_from_cookie_string_parses_unknown_as_map() {
        let s = "SESSDATA=abc; FEED_LIVE_VERSION=V8; bp_video_offset_123=456";
        let profile = AuthProfile::from_cookie_string(s);
        assert_eq!(profile.sessdata, "abc");
        assert_eq!(
            profile.cookies.get("FEED_LIVE_VERSION").map(String::as_str),
            Some("V8")
        );
    }

    #[test]
    fn test_merge_overwrites_non_empty() {
        let mut base = AuthProfile {
            sessdata: "old".to_string(),
            mid: 1,
            ..Default::default()
        };
        let other = AuthProfile {
            sessdata: "new".to_string(),
            bili_jct: "token".to_string(),
            mid: 2,
            ..Default::default()
        };
        base.merge(&other);
        assert_eq!(base.sessdata, "new");
        assert_eq!(base.bili_jct, "token");
        assert_eq!(base.mid, 2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut cookies = HashMap::new();
        cookies.insert("sid".to_string(), "x".to_string());
        let profile = AuthProfile {
            name: "default".to_string(),
            sessdata: "sess".to_string(),
            bili_jct: "jct".to_string(),
            buvid3: "buvid3".to_string(),
            cookies,
            ..Default::default()
        };
        let json = serde_json::to_string(&profile).unwrap();
        let parsed: AuthProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sessdata, "sess");
        assert_eq!(parsed.bili_jct, "jct");
        assert_eq!(parsed.cookies.get("sid").map(String::as_str), Some("x"));
    }

    #[test]
    fn test_default_is_empty() {
        let profile = AuthProfile::default();
        assert!(!profile.is_logged_in());
        assert!(profile.cookies.is_empty());
    }
}
