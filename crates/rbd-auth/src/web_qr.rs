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
    let url = data["url"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let qrcode_key = data["qrcode_key"]
        .as_str()
        .unwrap_or_default()
        .to_string();

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
/// 登录成功时从 HTTP redirect 响应头提取 cookies.
pub async fn poll(qrcode_key: &str) -> Result<HashMap<String, String>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
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

        let value = match poll_once(&client, &poll_url).await {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!("轮询失败, 重试: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let data = &value["data"];
        let code: i32 = data["code"].as_i64().unwrap_or(-1) as i32;

        match code {
            0 => {
                // 登录确认成功 — 跟随重定向 URL 获取 cookies
                let redirect_url = data["url"]
                    .as_str()
                    .ok_or_else(|| anyhow!("登录确认成功但缺少重定向 URL"))?;
                let cookies = extract_cookies_from_redirect(&client, redirect_url).await?;
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
async fn poll_once(
    client: &reqwest::Client,
    url: &str,
) -> Result<serde_json::Value> {
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

/// 跟随 B 站重定向 URL, 从 Set-Cookie 响应头提取 cookies.
async fn extract_cookies_from_redirect(
    client: &reqwest::Client,
    redirect_url: &str,
) -> Result<HashMap<String, String>> {
    let resp = client
        .get(redirect_url)
        .send()
        .await?;

    Ok(extract_cookies_from_headers(resp.headers()))
}

/// 从 HTTP 响应头解析 B 站 cookies.
pub(crate) fn extract_cookies_from_headers(headers: &reqwest::header::HeaderMap) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    let target_names = [
        "SESSDATA", "bili_jct", "DedeUserID", "buvid3", "buvid4",
        "dedeuserid", "sessdata", "ac_time_value",
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

/// 将二维码 URL 渲染到终端.
///
/// TODO: M5 使用 `qrcode` + `image` 生成真正的终端 QR 图.
pub fn render_qr_terminal(url: &str) {
    println!("\n请使用 B 站 App 扫描下方二维码登录:");
    println!("{url}");
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
        headers.insert(
            SET_COOKIE,
            HeaderValue::from_static("UNKNOWN_KEY=foo"),
        );
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
}
