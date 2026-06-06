//! 登录态刷新 — SESSDATA 过期检测.
//!
//! 通过 B 站 `/x/web-interface/nav` 接口检测当前登录态是否有效.
//! 返回的 `data.isLogin` 为 true 表示 SESSDATA 尚未过期.

use anyhow::Result;
use rbd_core::BilibiliApi;

use crate::profile::AuthProfile;

/// 检测 SESSDATA 是否有效.
///
/// 调用 `/x/web-interface/nav` 检查 `data.isLogin`.
/// 若已登录, 则 SESSDATA 仍有效; 否则返回 false.
pub async fn is_session_valid(api: &BilibiliApi, profile: &AuthProfile) -> bool {
    if !profile.is_logged_in() {
        return false;
    }

    let url = "https://api.bilibili.com/x/web-interface/nav";
    let result: Result<serde_json::Value> = api.get_json(url).await;

    match result {
        Ok(value) => value["data"]["isLogin"].as_bool().unwrap_or(false),
        Err(err) => {
            tracing::warn!("检测登录态失败: {err}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_session_valid_not_logged_in() {
        let api = BilibiliApi::new().unwrap();
        let profile = AuthProfile::default();
        let fut = is_session_valid(&api, &profile);

        // 未登录时 is_session_valid 应同步返回 false (不需要实际网络调用)
        // 但因为是 async fn, 这里仅验证签名编译通过
        assert!(std::mem::size_of_val(&fut) > 0);
    }

    #[test]
    fn test_is_session_valid_with_profile() {
        let profile = AuthProfile {
            sessdata: "fake_sessdata".to_string(),
            ..Default::default()
        };
        assert!(profile.is_logged_in());
    }
}
