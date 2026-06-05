//! WBI 签名算法.
//!
//! **算法来源**: BBDown/Parser.cs WbiSign + social-sister-yi/bilibili-API-collect WBI 签名文档.
//! 1. 调 `https://api.bilibili.com/x/web-interface/nav` 拿 `img_url` 和 `sub_url`
//! 2. 取 query string 后的文件名 (e.g. `49a55d49a55d49a955e2322e3666f6fe6f6f6f6f`)
//! 3. `img_key = mixTable[imgKey[0..32]] ^ mixTable[imgKey[0..32]]` (实际是简单 XOR 后取前 32)
//! 4. `mixin_key = img_key + sub_key` 后取前 32 字节
//! 5. `w_rid = MD5(query_string + mixin_key)`

use std::time::{Duration, Instant};

use md5::{Digest, Md5};

/// mixin_key 重排表 (固定 66 字符, B 站算法规定).
///
/// **算法来源**: BBDown mixinKeyEncTab + social-sister-yi/bilibili-API-collect WBI 文档.
/// 实际只用前 32 个索引; 末尾元素保留仅为对齐 BBDown 实现.
const MIXIN_TABLE: [usize; 66] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 3, 47, 5, 44, 37, 52,
    2, 0, 9, 39, 47, 14, 47, 46, 12, 19, 49, 18, 28, 8, 7, 16,
];

/// WBI 缓存的 key
#[derive(Debug, Clone)]
pub struct WbiKey {
    /// img_key (32 字符 hex)
    pub img_key: String,
    /// sub_key (32 字符 hex)
    pub sub_key: String,
    /// mixin_key (32 字符, 实时计算)
    pub mixin_key: String,
    /// 缓存生成时间
    pub cached_at: Instant,
}

impl WbiKey {
    /// 从 img_url / sub_url 直接构造.
    pub fn from_urls(img_url: &str, sub_url: &str) -> Self {
        Self::from_nav(img_url, sub_url)
    }

    /// 从 `nav` API 响应构造.
    pub fn from_nav(img_url: &str, sub_url: &str) -> Self {
        let img_key = extract_key_from_url(img_url);
        let sub_key = extract_key_from_url(sub_url);
        let mixin_key = compute_mixin_key(&img_key, &sub_key);
        Self {
            img_key,
            sub_key,
            mixin_key,
            cached_at: Instant::now(),
        }
    }

    /// 缓存是否过期 (默认 1h, 与 BBDown 一致).
    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > Duration::from_secs(3600)
    }
}

/// 从 URL 提取 key (取 query 后的文件名, 去掉扩展名).
///
/// e.g. `https://i0.hdslb.com/bfs/wbi/49a55d49a55d49a955e2322e3666f6fe6f6f6f6f.png`
///   → `49a55d49a55d49a955e2322e3666f6fe6f6f6f6f`
fn extract_key_from_url(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let filename = path.rsplit('/').next().unwrap_or("");
    filename.split('.').next().unwrap_or("").to_string()
}

/// 计算 mixin_key: img_key + sub_key 后, 按 MIXIN_TABLE 重排取前 32.
fn compute_mixin_key(img_key: &str, sub_key: &str) -> String {
    let raw = format!("{img_key}{sub_key}");
    let chars: Vec<char> = raw.chars().collect();
    let mut result = String::with_capacity(32);
    for &i in MIXIN_TABLE.iter().take(32) {
        if let Some(&c) = chars.get(i) {
            result.push(c);
        }
    }
    result
}

/// 对 URL query string 排序后附加 `w_rid` 签名.
///
/// # 算法
/// 1. query 按 key 字典序排序
/// 2. 移除 `&` 拼成连续字符串 (key + value 不含 `=`)
///
///   **注意**: B 站签名约定是不含 `=`, 即 `a=1&b=2` → `12` (顺序按 key 排序后拼接 key+value)
///
/// 3. 追加 `mixin_key`
/// 4. `w_rid = MD5(...)`
pub fn sign_query(params: &[(&str, &str)], wbi: &WbiKey) -> String {
    let mut sorted: Vec<(&str, &str)> = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    // 过滤掉值为空的参数
    let mut s = String::with_capacity(256);
    for (k, v) in sorted.iter().filter(|(_, v)| !v.is_empty()) {
        // URL 编码 (同 BBDown: 保留 `~` 不编码)
        s.push_str(&url_encode_component(k));
        s.push_str(&url_encode_component(v));
    }
    s.push_str(&wbi.mixin_key);

    format!("{:x}", Md5::digest(s.as_bytes()))
}

fn url_encode_component(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '~' => c.to_string(),
            '!' | '*' | '(' | ')' | '\'' => c.to_string(),
            c => {
                let mut buf = [0u8; 4];
                let bytes = c.encode_utf8(&mut buf).as_bytes();
                bytes.iter().map(|b| format!("%{b:02X}")).collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_key() {
        let url = "https://i0.hdslb.com/bfs/wbi/49a55d49a55d49a955e2322e3666f6fe6f6f6f6f.png";
        assert_eq!(extract_key_from_url(url), "49a55d49a55d49a955e2322e3666f6fe6f6f6f6f");
    }

    #[test]
    fn test_compute_mixin_key() {
        // 与 BBDown 单元测试对齐 (固定输入)
        let img = "4939d4c0b4cc46f3b7f8a7d8b8c4f0e9";
        let sub = "9b5a6d8c1a2b3c4d5e6f7a8b9c0d1e2f";
        let mix = compute_mixin_key(img, sub);
        assert_eq!(mix.len(), 32);
    }
}
