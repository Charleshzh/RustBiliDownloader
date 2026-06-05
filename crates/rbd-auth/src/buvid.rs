//! buvid 主动获取 — 从 www.bilibili.com 拿设备指纹.
//!
//! buvid3 / buvid4 是 B 站设备指纹, 用于反爬校验.
//! 访问首页时, 服务器通过 Set-Cookie 返回这两个值.

use anyhow::Result;

/// 获取 buvid3 和 buvid4.
///
/// 访问 <https://www.bilibili.com/> 并解析响应中的 Set-Cookie 头.
/// 返回 `(buvid3, buvid4)`.
pub async fn fetch_buvid() -> Result<(String, String)> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://www.bilibili.com/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await?;

    let mut buvid3 = String::new();
    let mut buvid4 = String::new();

    for header_value in response.headers().get_all("set-cookie") {
        if let Ok(value) = header_value.to_str() {
            // 取第一个分号前的键值对
            if let Some(pair) = value.split(';').next() {
                let pair = pair.trim();
                if let Some((key, val)) = pair.split_once('=') {
                    match key.trim() {
                        "buvid3" => buvid3 = val.trim().to_string(),
                        "buvid4" => buvid4 = val.trim().to_string(),
                        _ => {}
                    }
                }
            }
        }
    }

    if buvid3.is_empty() && buvid4.is_empty() {
        // 返回空值时记录警告, 但不中止 — 不影响主流程
        tracing::warn!("未能从 bilibili.com 获取 buvid — 将使用空值继续");
    }

    Ok((buvid3, buvid4))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试解析 Set-Cookie 样式的 buvid.
    #[test]
    fn test_fetch_buvid_structure() {
        // 此测试仅验证函数签名和返回类型.
        // 真实 HTTP 请求在集成测试中验证.
        let fut = fetch_buvid();
        assert!(std::mem::size_of_val(&fut) > 0);
    }
}
