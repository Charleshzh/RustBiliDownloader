//! B 站 WEB API 客户端.

use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;
use reqwest::{
    header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT},
    Client,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::wbi::{sign_query, WbiKey};

/// B站 playurl 请求参数, 同时启用 DASH + HDR + 4K + 杜比 + 8K + AV1.
pub const FNVAL_DASH_ALL: &str = "4048";

/// B 站 WEB API 客户端 (可 Clone, 内部共享连接池和 WBI 缓存).
#[derive(Clone)]
pub struct BilibiliApi {
    client: Arc<Client>,
    wbi_key: Arc<Mutex<Option<WbiKey>>>,
}

#[derive(Debug, Deserialize)]
struct NavResp {
    data: NavData,
}

#[derive(Debug, Deserialize)]
struct NavData {
    wbi_img: NavImg,
    #[serde(default)]
    _mid: u64,
    #[serde(default)]
    _uname: String,
}

#[derive(Debug, Deserialize)]
struct NavImg {
    img_url: String,
    sub_url: String,
}

impl BilibiliApi {
    /// 创建 API 客户端.
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Arc::new(build_client(None)?),
            wbi_key: Arc::new(Mutex::new(None)),
        })
    }

    /// 设置 SESSDATA cookie (返回新实例, 不修改 self).
    #[must_use]
    pub fn with_cookie(&self, sessdata: &str) -> Result<Self> {
        Ok(Self {
            client: Arc::new(build_client(Some(sessdata))?),
            wbi_key: Arc::clone(&self.wbi_key),
        })
    }

    /// 设置 SESSDATA cookie (修改当前实例, 保留用于兼容).
    pub fn set_cookie(&mut self, sessdata: &str) -> Result<()> {
        self.client = Arc::new(build_client(Some(sessdata))?);
        Ok(())
    }

    /// 刷新 WBI key.
    pub async fn refresh_wbi_key(&self) -> Result<WbiKey> {
        let nav: NavResp = self.get_json("https://api.bilibili.com/x/web-interface/nav").await?;
        let key = WbiKey::from_urls(&nav.data.wbi_img.img_url, &nav.data.wbi_img.sub_url);
        *self.wbi_key.lock() = Some(key.clone());
        Ok(key)
    }

    /// 获取缓存中的 WBI key.
    pub fn cached_wbi_key(&self) -> Option<WbiKey> {
        self.wbi_key.lock().clone()
    }

    /// GET 并反序列化 JSON.
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        Ok(self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }

    /// 获取 view 响应.
    pub async fn get_view(&self, bvid: &str) -> Result<serde_json::Value> {
        self.get_json(&format!(
            "https://api.bilibili.com/x/web-interface/view?bvid={bvid}"
        ))
        .await
    }

    /// 获取 pagelist 响应.
    pub async fn get_pagelist(&self, bvid: &str) -> Result<serde_json::Value> {
        self.get_json(&format!(
            "https://api.bilibili.com/x/player/pagelist?bvid={bvid}&jsonp=jsonp"
        ))
        .await
    }

    /// 获取 playurl 响应.
    pub async fn get_playurl(&self, bvid: &str, cid: u64, qn: u32) -> Result<serde_json::Value> {
        self.get_json(&format!(
            "https://api.bilibili.com/x/player/playurl?bvid={bvid}&cid={cid}&qn={qn}&fnval={FNVAL_DASH_ALL}&fnver=0&fourk=1&platform=html5&high_quality=1"
        ))
        .await
    }

    /// 获取字幕响应.
    pub async fn get_subtitles(&self, bvid: &str, cid: u64) -> Result<serde_json::Value> {
        let url = self
            .build_wbi_signed_url(
                "https://api.bilibili.com/x/player/wbi/v2",
                vec![("bvid", bvid.to_string()), ("cid", cid.to_string())],
            )
            .await?;
        self.get_json(&url).await
    }

    /// 获取番剧 season 响应.
    pub async fn get_bangumi_season(&self, season_id: u64) -> Result<serde_json::Value> {
        self.get_json(&format!(
            "https://api.bilibili.com/pgc/view/web/season?season_id={season_id}"
        ))
        .await
    }

    /// 获取番剧单集响应.
    pub async fn get_bangumi_ep(&self, ep_id: u64) -> Result<serde_json::Value> {
        self.get_json(&format!("https://api.bilibili.com/pgc/view/web/ep?ep_id={ep_id}"))
            .await
    }

    /// 获取空间投稿列表.
    pub async fn get_space_archives(&self, mid: u64, page: u32) -> Result<serde_json::Value> {
        let url = self
            .build_wbi_signed_url(
                "https://api.bilibili.com/x/space/wbi/arc/search",
                vec![
                    ("mid", mid.to_string()),
                    ("pn", page.to_string()),
                    ("ps", "30".to_string()),
                ],
            )
            .await?;
        self.get_json(&url).await
    }

    /// 获取合集信息.
    pub async fn get_series(&self, series_id: u64) -> Result<serde_json::Value> {
        self.get_json(&format!(
            "https://api.bilibili.com/x/series/series?series_id={series_id}"
        ))
        .await
    }

    /// 获取收藏夹信息.
    pub async fn get_fav_folder(&self, fid: u64, _page: u32) -> Result<serde_json::Value> {
        self.get_json(&format!(
            "https://api.bilibili.com/x/v2/fav/folder/created/list-all?fid={fid}"
        ))
        .await
    }

    /// 获取课程信息.
    pub async fn get_cheese_season(&self, season_id: u64) -> Result<serde_json::Value> {
        self.get_json(&format!(
            "https://api.bilibili.com/pugv/view/web/season?season_id={season_id}"
        ))
        .await
    }

    async fn build_wbi_signed_url(&self, base: &str, params: Vec<(&str, String)>) -> Result<String> {
        let key = match self.cached_wbi_key() {
            Some(wbi) if !wbi.is_expired() => wbi,
            _ => self.refresh_wbi_key().await?,
        };
        Ok(build_wbi_signed_url(base, params, &key))
    }
}

fn build_client(sessdata: Option<&str>) -> Result<Client> {
    let headers = build_headers(sessdata)?;

    Ok(Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

fn build_headers(sessdata: Option<&str>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        ),
    );
    headers.insert(REFERER, HeaderValue::from_static("https://www.bilibili.com"));
    if let Some(sessdata) = sessdata {
        let value = format!("SESSDATA={sessdata}");
        headers.insert(COOKIE, HeaderValue::from_str(&value)?);
    }

    Ok(headers)
}

fn build_wbi_signed_url(base: &str, params: Vec<(&str, String)>, wbi: &WbiKey) -> String {
    let mut query: Vec<(&str, String)> = params;
    query.push(("wts", chrono::Utc::now().timestamp().to_string()));

    let for_sign: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let w_rid = sign_query(&for_sign, wbi);

    let encoded = query
        .into_iter()
        .map(|(k, v)| format!("{k}={}", urlencoding::encode(&v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{base}?{encoded}&w_rid={w_rid}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wbi_nav_json() {
        let sample = r#"{
            "data": {
                "wbi_img": {
                    "img_url": "https://i0.hdslb.com/bfs/wbi/abc123.png",
                    "sub_url": "https://i0.hdslb.com/bfs/wbi/def456.png"
                },
                "mid": 42,
                "uname": "tester"
            }
        }"#;

        let parsed: NavResp = serde_json::from_str(sample).unwrap();
        assert_eq!(parsed.data.wbi_img.img_url, "https://i0.hdslb.com/bfs/wbi/abc123.png");
        assert_eq!(parsed.data.wbi_img.sub_url, "https://i0.hdslb.com/bfs/wbi/def456.png");
    }

    #[tokio::test]
    async fn test_set_cookie() {
        let headers = build_headers(Some("sess-token")).unwrap();

        assert_eq!(
            headers.get(COOKIE).unwrap(),
            &HeaderValue::from_static("SESSDATA=sess-token")
        );
    }

    #[test]
    fn test_build_wbi_signed_url_contains_signature() {
        let wbi = WbiKey::from_urls(
            "https://i0.hdslb.com/bfs/wbi/4939d4c0b4cc46f3b7f8a7d8b8c4f0e9.png",
            "https://i0.hdslb.com/bfs/wbi/9b5a6d8c1a2b3c4d5e6f7a8b9c0d1e2f.png",
        );
        let url = build_wbi_signed_url(
            "https://api.bilibili.com/x/player/wbi/v2",
            vec![("bvid", "BV1xx411c7mD".to_string()), ("cid", "123".to_string())],
            &wbi,
        );

        assert!(url.starts_with("https://api.bilibili.com/x/player/wbi/v2?"));
        assert!(url.contains("bvid=BV1xx411c7mD"));
        assert!(url.contains("cid=123"));
        assert!(url.contains("wts="));
        assert!(url.contains("w_rid="));
    }

    #[test]
    fn test_bilibili_api_clone() {
        let _api = BilibiliApi::new().unwrap();
        // clone 不应 panic
        let _api2 = _api.clone();
    }
}
