//! WEB 扫码登录 — 生成二维码 + 轮询状态 + 终端渲染.
//!
//! **算法来源**: BBDown `BBDownLoginUtil` + Yutto `login.py`.
//! **端点** (B 站 Passport):
//! - `POST /x/passport-login/web/qrcode/generate` → qrcode_key + url
//! - `GET /x/passport-login/web/qrcode/poll?qrcode_key=` → 状态

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use rbd_core::BilibiliApi;
use serde::{Deserialize, Serialize};

/// 二维码生成响应.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrGenerateResponse {
    /// 二维码内容 URL.
    pub url: String,
    /// 轮询 key.
    pub qrcode_key: String,
}

/// 二维码轮询响应 (内部使用, 不再直接暴露).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrPollResponse {
    /// 扫码状态.
    pub status: QrStatus,
    /// 可读消息.
    pub message: String,
    /// 登录成功时返回的 cookie.
    pub cookies: HashMap<String, String>,
}

/// 二维码状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QrStatus {
    /// 未扫码.
    NotScanned,
    /// 已扫码待确认.
    Scanned,
    /// 已确认.
    Confirmed,
    /// 二维码已过期.
    Expired,
    /// 未知状态.
    Unknown,
}

/// 生成 WEB 登录二维码.
///
/// 返回二维码 URL 和轮询 key.
pub async fn generate() -> Result<QrGenerateResponse> {
    let api = BilibiliApi::new()?;
    let value: serde_json::Value = api
        .get_json("https://passport.bilibili.com/x/passport-login/web/qrcode/generate")
        .await?;

    let data = &value["data"];
    let url = data["url"].as_str().unwrap_or_default().to_string();
    let qrcode_key = data["qrcode_key"].as_str().unwrap_or_default().to_string();

    if qrcode_key.is_empty() {
        return Err(anyhow!(
            "获取登录二维码失败: {}",
            value["message"].as_str().unwrap_or("未知错误")
        ));
    }

    Ok(QrGenerateResponse { url, qrcode_key })
}

/// 轮询二维码扫码状态 (带循环 + 超时).
///
/// 每 2 秒轮询一次, 最长等待 180 秒.
/// 登录成功时通过 cookie store 跟随重定向链收集 cookies.
pub async fn poll(qrcode_key: &str) -> Result<HashMap<String, String>> {
    // 用于轮询的轻量 client (no cookie store needed)
    let poll_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // 用于跟随登录重定向链收集 cookie 的 client.
    // CRITICAL: 禁用自动重定向 — 必须手动逐跳处理, 因为 reqwest 默认
    // 自动跟随重定向会跳过中间响应, 导致 Set-Cookie (SESSDATA 等) 全部丢失.
    let login_client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let poll_url = format!(
        "https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={qrcode_key}"
    );

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(180);
    let interval = std::time::Duration::from_secs(2);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("登录超时 (180 秒), 请重新生成二维码");
        }

        let value = match poll_once(&poll_client, &poll_url).await {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!("轮询失败, 重试: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let data = &value["data"];
        let code: i32 = i32::try_from(data["code"].as_i64().unwrap_or(-1)).unwrap_or(-1);

        match code {
            0 => {
                // 登录确认成功 — 跟随重定向链收集 cookies
                let redirect_url = data["url"]
                    .as_str()
                    .ok_or_else(|| anyhow!("登录确认成功但缺少重定向 URL"))?;
                let cookies =
                    collect_login_cookies(&login_client, redirect_url.to_string()).await?;
                tracing::info!("WEB 登录成功, 获取到 {} 个 cookie", cookies.len());
                return Ok(cookies);
            }
            86038 => anyhow::bail!("二维码已过期, 请重新生成"),
            86090 => {
                tracing::info!("已扫码, 等待手机确认...");
            }
            86101 => {
                // 未扫码, 静默等待
            }
            _ => {
                tracing::debug!(
                    "轮询返回未知状态: code={}, message={}",
                    code,
                    data["message"].as_str().unwrap_or("未知")
                );
            }
        }

        tokio::time::sleep(interval).await;
    }
}

/// 单次轮询 (内部).
async fn poll_once(client: &reqwest::Client, url: &str) -> Result<serde_json::Value> {
    let resp = client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await?;
    Ok(resp.json().await?)
}

/// 手动跟随 B 站登录重定向链, 逐跳收集 Set-Cookie.
///
/// 请求 `redirect_url` → 检查响应 → 如果是 302/301, 从 Set-Cookie 提取,
/// 跟随 Location → 循环直到 200 或非重定向状态码.
///
/// 额外兜底: 从原始重定向 URL 的 query 参数提取 `SESSDATA/bili_jct/DedeUserID`,
/// 因为 B 站 crossDomain 端点把这些值同时放在 URL query 和 Set-Cookie 中.
async fn collect_login_cookies(
    client: &reqwest::Client,
    redirect_url: String,
) -> Result<HashMap<String, String>> {
    // 兜底 #1: 从 URL query 参数提取 SESSDATA/bili_jct/DedeUserID
    let mut all_cookies = parse_auth_query_params(&redirect_url);

    let max_hops = 10;
    let mut current_url = redirect_url;
    let base_url = reqwest::Url::parse(&current_url)
        .unwrap_or_else(|_| reqwest::Url::parse("https://www.bilibili.com").unwrap());

    for hop in 0..max_hops {
        let resp = client
            .get(&current_url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        // 从当前响应提取 Set-Cookie
        let cookies = extract_cookies_from_headers(resp.headers());
        all_cookies.extend(cookies);

        let status = resp.status();
        tracing::debug!(
            "重定向 {hop}: {status} ← {current_url}, 收集 {} cookies",
            all_cookies.len()
        );

        // 检查是否要跟随 Location
        if status.is_redirection() {
            if let Some(location) = resp.headers().get(reqwest::header::LOCATION) {
                let loc = location
                    .to_str()
                    .map_err(|_| anyhow!("Location 头解析失败"))?;
                // 处理相对 URL: 绝对化
                current_url = if loc.starts_with("http") {
                    loc.to_string()
                } else {
                    base_url
                        .join(loc)
                        .map_err(|e| anyhow!("Location URL 拼接失败: {e}"))?
                        .to_string()
                };
                continue;
            }
        }
        // 非重定向, 结束
        break;
    }

    if all_cookies.is_empty() {
        return Err(anyhow!("未能从登录重定向链收集到任何 cookie"));
    }

    Ok(all_cookies)
}

/// 从 URL query 参数解析 `SESSDATA/bili_jct/DedeUserID`.
///
/// B 站 crossDomain 端点把这些认证 cookie 同时放在 URL query 和 Set-Cookie 中,
/// 作为兜底方案直接解析.
fn parse_auth_query_params(redirect_url: &str) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    let target_keys = ["SESSDATA", "bili_jct", "DedeUserID", "buvid3"];

    // 尝试解析 URL
    if let Ok(url) = reqwest::Url::parse(redirect_url) {
        for (key, value) in url.query_pairs() {
            if target_keys.iter().any(|k| k.eq_ignore_ascii_case(&key)) {
                cookies.insert(key.to_string(), value.to_string());
            }
        }
    }
    cookies
}

/// 从 HTTP 响应头解析 B 站 cookies.
fn extract_cookies_from_headers(headers: &reqwest::header::HeaderMap) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    let target_names = [
        "SESSDATA",
        "bili_jct",
        "DedeUserID",
        "buvid3",
        "buvid4",
        "dedeuserid",
        "sessdata",
        "ac_time_value",
    ];

    for value in headers.get_all(reqwest::header::SET_COOKIE) {
        if let Ok(cookie_str) = value.to_str() {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some((key, val)) = part.split_once('=') {
                    let key = key.trim();
                    if target_names.iter().any(|n| n.eq_ignore_ascii_case(key)) {
                        cookies.insert(key.to_string(), val.trim().to_string());
                    }
                }
            }
        }
    }
    cookies
}

/// 将二维码 URL 渲染到终端 (Unicode 字符块).
///
/// 使用 `qrcode` crate 生成真正的终端 QR 图,
/// 支持所有支持 Unicode 的终端.
pub fn render_qr_terminal(url: &str) {
    use qrcode::QrCode;
    use qrcode::render::unicode;

    let code = match QrCode::new(url) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("QR 生成失败: {e}");
            println!("请手动访问: {url}");
            return;
        }
    };

    println!();
    println!("请使用 B 站 App 扫描下方二维码登录:");
    println!();
    // Unicode 块渲染: ▀▄█▌ 高对比度
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Dark)
        .light_color(unicode::Dense1x2::Light)
        .build();
    println!("{image}");
    println!();
    println!("如果二维码无法显示, 请手动访问: {url}");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_generate_response_serde() {
        let resp = QrGenerateResponse {
            url: "https://login.bilibili.com/qr".to_string(),
            qrcode_key: "abc123".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: QrGenerateResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, "https://login.bilibili.com/qr");
        assert_eq!(parsed.qrcode_key, "abc123");
    }

    #[test]
    fn test_qr_poll_response_serde() {
        let resp = QrPollResponse {
            status: QrStatus::Confirmed,
            message: "登录成功".to_string(),
            cookies: HashMap::new(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: QrPollResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, QrStatus::Confirmed);
    }

    #[test]
    fn test_qr_status_serde() {
        assert_eq!(
            serde_json::to_string(&QrStatus::NotScanned).unwrap(),
            r#""notscanned""#
        );
        assert_eq!(
            serde_json::to_string(&QrStatus::Confirmed).unwrap(),
            r#""confirmed""#
        );
    }

    #[test]
    fn test_extract_cookies_from_headers_parses_set_cookie() {
        use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
        let mut headers = HeaderMap::new();
        headers.insert(
            SET_COOKIE,
            HeaderValue::from_static("SESSDATA=abc123; Domain=.bilibili.com; Path=/"),
        );
        let cookies = extract_cookies_from_headers(&headers);
        assert_eq!(cookies.get("SESSDATA").map(String::as_str), Some("abc123"));
    }

    #[test]
    fn test_extract_cookies_filters_unknown_names() {
        use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
        let mut headers = HeaderMap::new();
        headers.insert(SET_COOKIE, HeaderValue::from_static("UNKNOWN_KEY=foo"));
        let cookies = extract_cookies_from_headers(&headers);
        assert!(cookies.is_empty());
    }

    #[test]
    fn test_render_qr_terminal_does_not_panic() {
        render_qr_terminal("https://login.bilibili.com/qr");
    }

    #[test]
    fn test_qr_poll_response_error_json() {
        let json = r#"{"status":"expired","message":"已过期","cookies":{}}"#;
        let parsed: QrPollResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.status, QrStatus::Expired);
    }

    #[test]
    fn test_parse_auth_query_params_from_cross_domain_url() {
        let url = "https://passport.bilibili.com/x/passport-login/web/crossDomain?DedeUserID=12345&DedeUserID__ckMd5=abc&SESSDATA=deadbeef&bili_jct=xyz123&gourl=https%3A%2F%2Fwww.bilibili.com";
        let cookies = parse_auth_query_params(url);
        assert_eq!(
            cookies.get("SESSDATA").map(String::as_str),
            Some("deadbeef")
        );
        assert_eq!(cookies.get("bili_jct").map(String::as_str), Some("xyz123"));
        assert_eq!(cookies.get("DedeUserID").map(String::as_str), Some("12345"));
    }

    #[test]
    fn test_parse_auth_query_params_empty() {
        let cookies = parse_auth_query_params("https://www.bilibili.com/");
        assert!(cookies.is_empty());
    }
}
