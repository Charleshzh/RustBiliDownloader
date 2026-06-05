//! HTTP Range 客户端.

use anyhow::Result;
use bytes::Bytes;
use reqwest::header::{HeaderMap, ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, RANGE};

/// 单次 Range 请求响应.
#[derive(Debug, Clone)]
pub struct RangeResponse {
    /// 响应数据.
    pub data: Bytes,
    /// 起始偏移.
    pub start: u64,
    /// 结束偏移.
    pub end: u64,
    /// 总大小.
    pub total: u64,
}

/// HTTP Range 客户端.
#[derive(Clone)]
pub struct RangeClient {
    client: reqwest::Client,
}

impl RangeClient {
    /// 创建默认客户端.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// 使用外部客户端.
    #[must_use]
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// HEAD 获取总长度.
    pub async fn head(&self, url: &str, headers: &HeaderMap) -> Result<u64> {
        let response = self
            .client
            .head(url)
            .headers(headers.clone())
            .send()
            .await?
            .error_for_status()?;
        Ok(response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_default())
    }

    /// 获取指定 Range.
    pub async fn get_range(
        &self,
        url: &str,
        start: u64,
        end: u64,
        headers: &HeaderMap,
    ) -> Result<RangeResponse> {
        let mut req_headers = headers.clone();
        req_headers.insert(RANGE, format!("bytes={start}-{end}").parse()?);
        let response = self
            .client
            .get(url)
            .headers(req_headers)
            .send()
            .await?
            .error_for_status()?;
        let total = parse_total(response.headers()).unwrap_or(end.saturating_add(1));
        let data = response.bytes().await?;
        Ok(RangeResponse {
            data,
            start,
            end,
            total,
        })
    }

    /// 是否支持 Range.
    pub async fn supports_range(&self, url: &str, headers: &HeaderMap) -> bool {
        match self.client.head(url).headers(headers.clone()).send().await {
            Ok(response) => response
                .headers()
                .get(ACCEPT_RANGES)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.eq_ignore_ascii_case("bytes")),
            Err(_) => false,
        }
    }
}

impl Default for RangeClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_total(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit('/').next())
        .and_then(|value| value.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::RangeClient;

    #[test]
    fn test_range_client_new() {
        let _client = RangeClient::new();
    }
}
