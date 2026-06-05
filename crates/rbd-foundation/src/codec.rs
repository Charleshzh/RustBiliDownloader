//! 通用编解码 (URL, Base58, hex, UTF-8 校验).
//!
//! **算法来源**: BV 转换用 Base58 (B 站固定表), 详见 rbd-core::bv.

/// B 站 BV 转换用的 Base58 字符表.
pub const BV_ALPHABET: &[u8; 58] = b"FcwAPNKTMug3GV5Lj7EJnHpWsx4tb8haYeviqBz6rkCy12mUSDQX9RdoZf";

/// 十六进制编码 (小写).
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 十六进制解码.
pub fn hex_decode(s: &str) -> anyhow::Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        anyhow::bail!("奇数长度 hex 字符串");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(Into::into))
        .collect()
}

/// URL 编码 (保留 `~_-!.*'()` 不编码, 模拟 JS `encodeURIComponent`).
pub fn url_encode_component(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '~' | '!' | '*' | '(' | ')' | '\'' => c.to_string(),
            c => {
                let mut buf = [0u8; 4];
                let bytes = c.encode_utf8(&mut buf).as_bytes();
                bytes.iter().map(|b| format!("%{b:02X}")).collect()
            }
        })
        .collect()
}

/// URL 解码.
pub fn url_decode_component(s: &str) -> anyhow::Result<String> {
    percent_encoding::percent_decode_str(s)
        .decode_utf8()
        .map(|c| c.into_owned())
        .map_err(anyhow::Error::from)
}

/// Base58 编码 (B 站固定表).
pub fn base58_encode(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut leading_zeros = 0;
    for &b in bytes {
        if b == 0 {
            leading_zeros += 1;
        } else {
            break;
        }
    }
    let mut n: u64 = bytes.iter().fold(0u64, |acc, &b| (acc << 8) | b as u64);
    while n > 0 {
        let r = (n % 58) as usize;
        n /= 58;
        result.insert(0, BV_ALPHABET[r] as char);
    }
    for _ in 0..leading_zeros {
        result.insert(0, BV_ALPHABET[0] as char);
    }
    result
}

/// MD5 哈希 (Hex 字符串).
pub fn md5_hex(bytes: &[u8]) -> String {
    use md5::{Md5, Digest};
    let mut h = Md5::new();
    h.update(bytes);
    hex_encode(&h.finalize())
}

/// SHA1 哈希.
pub fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(bytes);
    hex_encode(&h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_round_trip() {
        let data = b"hello world";
        let enc = hex_encode(data);
        let dec = hex_decode(&enc).unwrap();
        assert_eq!(data, dec.as_slice());
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode_component("hello world"), "hello%20world");
        assert_eq!(url_encode_component("a=b&c=d"), "a%3Db%26c%3Dd");
        assert_eq!(url_encode_component("hello-world_~"), "hello-world_~");
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode_component("hello%20world").unwrap(), "hello world");
        assert_eq!(url_decode_component("a%3Db").unwrap(), "a=b");
    }

    #[test]
    fn test_md5() {
        // 已知测试向量
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn test_sha1() {
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }
}
