//! TV 扫码登录 — TV 端专用端点 (备选 8K 路径).
//!
//! **端点**: `https://passport.snm0516.aisee.tv/x/passport-tv-login/qrcode/auth_code`
//!
//! TV 登录流程分三步:
//! 1. `generate()` — 获取 auth_code + 网页确认 URL
//! 2. 用户在网页输入 auth_code 后点确认
//! 3. `poll()` — 轮询确认结果, 返回 cookies (带循环 + 180s 超时)

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use rbd_core::BilibiliApi;

use crate::web_qr::QrGenerateResponse;

/// 生成 TV 登录二维码.
///
/// 返回 `auth_code` 供用户在网页上输入.
pub async fn generate() -> Result<QrGenerateResponse> {
    let api = BilibiliApi::new()?;
    let value: serde_json::Value = api
        .get_json("https://passport.snm0516.aisee.tv/x/passport-tv-login/qrcode/auth_code")
        .await?;

    let data = &value["data"];
    let url = data["url"].as_str().unwrap_or_default().to_string();
    let auth_code = data["auth_code"].as_str().unwrap_or_default().to_string();

    if auth_code.is_empty() {
        return Err(anyhow!(
            "获取 TV 登录二维码失败: {}",
            value["message"].as_str().unwrap_or("未知错误")
        ));
    }

    Ok(QrGenerateResponse {
        url,
        qrcode_key: auth_code,
    })
}

/// 轮询 TV 登录结果 (带循环 + 180s 超时).
///
/// 每 2 秒轮询一次, 最长等待 180 秒.
/// 登录成功时从响应头提取 cookies.
pub async fn poll(auth_code: &str) -> Result<HashMap<String, String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let poll_url = format!(
        "https://passport.snm0516.aisee.tv/x/passport-tv-login/qrcode/poll?auth_code={auth_code}"
    );

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(180);
    let interval = std::time::Duration::from_secs(2);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("TV 登录超时 (180 秒), 请重新生成二维码");
        }

        let resp = match client
            .get(&poll_url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("轮询请求失败, 重试: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let value: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!("JSON 解析失败, 重试: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let data = &value["data"];
        let code: i32 = data["code"].as_i64().unwrap_or(-1) as i32;

        match code {
            0 => {
                // TV 登录确认成功 — cookies 在 JSON body 中返回
                let mut cookies = HashMap::new();
                for field in &["sessdata", "bili_jct", "dedeuserid", "ac_time_value"] {
                    if let Some(val) = data[*field].as_str() {
                        cookies.insert((*field).to_string(), val.to_string());
                    }
                }

                if cookies.is_empty() {
                    anyhow::bail!("TV 登录确认成功但未获取到 cookies");
                }

                tracing::info!("TV 登录成功, 获取到 {} 个 cookie", cookies.len());
                return Ok(cookies);
            }
            86038 => anyhow::bail!("TV 登录二维码已过期, 请重新生成"),
            86039 => {
                tracing::info!("已确认, 等待服务器处理...");
            }
            _ => {
                tracing::debug!(
                    "TV 轮询返回: code={}, message={}",
                    code,
                    data["message"].as_str().unwrap_or("等待确认中...")
                );
            }
        }

        tokio::time::sleep(interval).await;
    }
}

/// 用户在网页输入 `auth_code` 后确认登录.
///
/// 此函数供用户在网页端输入 code 后调用, 返回登录 cookies.
pub async fn confirm(auth_code: &str, code: &str) -> Result<HashMap<String, String>> {
    let payload = serde_json::json!({
        "auth_code": auth_code,
        "code": code,
    });
    let client = reqwest::Client::new();
    let value: serde_json::Value = client
        .post("https://passport.snm0516.aisee.tv/x/passport-tv-login/qrcode/confirm")
        .json(&payload)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let code_val: i32 = value["code"].as_i64().unwrap_or(-1) as i32;
    if code_val != 0 {
        return Err(anyhow!(
            "TV 登录确认失败: {}",
            value["message"].as_str().unwrap_or("未知错误")
        ));
    }

    let mut cookies = HashMap::new();
    let data = &value["data"];
    for field in &["sessdata", "bili_jct", "dedeuserid", "ac_time_value"] {
        if let Some(val) = data[*field].as_str() {
            cookies.insert((*field).to_string(), val.to_string());
        }
    }

    Ok(cookies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web_qr::{QrPollResponse, QrStatus};

    #[test]
    fn test_qr_generate_response_for_tv() {
        let resp = QrGenerateResponse {
            url: "https://tv.bilibili.com/qr".to_string(),
            qrcode_key: "tv_auth_001".to_string(),
        };
        assert_eq!(resp.qrcode_key, "tv_auth_001");
    }

    #[test]
    fn test_qr_poll_response_for_tv_confirmed() {
        let mut cookies = HashMap::new();
        cookies.insert("sessdata".to_string(), "tv_sess".to_string());
        let resp = QrPollResponse {
            status: QrStatus::Confirmed,
            message: "确认成功".to_string(),
            cookies,
        };
        assert_eq!(resp.status, QrStatus::Confirmed);
        assert_eq!(
            resp.cookies.get("sessdata").map(String::as_str),
            Some("tv_sess")
        );
    }

    #[test]
    fn test_qr_poll_response_error_json() {
        let json = r#"{"status":"expired","message":"已过期","cookies":{}}"#;
        let parsed: QrPollResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.status, QrStatus::Expired);
    }
}
