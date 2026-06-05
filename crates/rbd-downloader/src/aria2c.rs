//! aria2c JSON-RPC 客户端.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// aria2c 客户端.
#[derive(Debug, Clone)]
pub struct Aria2cClient {
    endpoint: String,
    secret: Option<String>,
}

impl Aria2cClient {
    /// 创建客户端.
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            secret: None,
        }
    }

    /// 设置 token.
    #[must_use]
    pub fn with_secret(mut self, s: impl Into<String>) -> Self {
        self.secret = Some(s.into());
        self
    }

    /// 获取 endpoint.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// 添加下载任务.
    pub async fn add_uri(&self, uri: &str, options: serde_json::Value) -> Result<String> {
        let params = if let Some(secret) = &self.secret {
            serde_json::json!([[format!("token:{secret}")], [uri], options])
        } else {
            serde_json::json!([[uri], options])
        };
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "rbd",
            "method": "aria2.addUri",
            "params": params,
        });
        let value = rpc_call(&self.endpoint, payload).await?;
        value
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| anyhow!("aria2.addUri 缺少 result"))
    }

    /// 查询任务状态.
    pub async fn tell_status(&self, gid: &str) -> Result<Aria2Status> {
        let params = if let Some(secret) = &self.secret {
            serde_json::json!([format!("token:{secret}"), gid])
        } else {
            serde_json::json!([gid])
        };
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "rbd",
            "method": "aria2.tellStatus",
            "params": params,
        });
        let value = rpc_call(&self.endpoint, payload).await?;
        Ok(serde_json::from_value(value["result"].clone())?)
    }

    /// 删除任务.
    pub async fn remove(&self, gid: &str) -> Result<()> {
        let params = if let Some(secret) = &self.secret {
            serde_json::json!([format!("token:{secret}"), gid])
        } else {
            serde_json::json!([gid])
        };
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "rbd",
            "method": "aria2.remove",
            "params": params,
        });
        let _ = rpc_call(&self.endpoint, payload).await?;
        Ok(())
    }
}

impl Default for Aria2cClient {
    fn default() -> Self {
        Self::new("http://localhost:6800/jsonrpc")
    }
}

async fn rpc_call(endpoint: &str, payload: serde_json::Value) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    Ok(client
        .post(endpoint)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

/// aria2 状态.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aria2Status {
    /// gid.
    pub gid: String,
    /// 状态.
    pub status: String,
    /// 总大小.
    #[serde(deserialize_with = "from_string_or_number")]
    pub total_length: u64,
    /// 已完成.
    #[serde(deserialize_with = "from_string_or_number")]
    pub completed_length: u64,
    /// 当前速度.
    #[serde(deserialize_with = "from_string_or_number")]
    pub download_speed: u64,
}

fn from_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Number(u64),
        Text(String),
    }

    match Value::deserialize(deserializer)? {
        Value::Number(value) => Ok(value),
        Value::Text(value) => value.parse::<u64>().map_err(serde::de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::{Aria2Status, Aria2cClient};

    #[test]
    fn test_aria2c_client_new() {
        let client = Aria2cClient::default();
        assert_eq!(client.endpoint(), "http://localhost:6800/jsonrpc");
    }

    #[test]
    fn test_aria2c_status_serialize() {
        let status = Aria2Status {
            gid: "gid".to_string(),
            status: "active".to_string(),
            total_length: 100,
            completed_length: 50,
            download_speed: 10,
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: Aria2Status = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.gid, "gid");
        assert_eq!(parsed.completed_length, 50);
    }
}
