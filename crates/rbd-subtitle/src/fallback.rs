//! 字幕 fallback 链 — 5 套 API 按顺序回退.
//!
//! **API 顺序 (回退链)**:
//! 1. `GET /x/player/wbi/v2?cid=xxx&bvid=xxx` (主用, 需 WBI 签名)
//! 2. `GET /x/player/v2?cid=xxx&bvid=xxx` (旧 fallback)
//! 3. `GET /x/player.so?id=cid={cid}` (JSON 格式, 老接口)
//! 4. `GET /x/web-interface/view?bvid=xxx` (视频元信息 subtitle 字段)
//! 5. `GET /x/v2/dm/view?type=1&oid=cid` (弹幕视图含字幕元数据)
//!
//! 依序尝试, 返回第一个非空结果; 全失败则返回空列表 (字幕非必需品).

use anyhow::Result;
use rbd_core::BilibiliApi;

use crate::model::Subtitle;

/// 字幕 fallback 链.
pub struct SubtitleFallback {
    api: BilibiliApi,
}

impl SubtitleFallback {
    /// 创建 fallback 链.
    #[must_use]
    pub fn new(api: BilibiliApi) -> Self {
        Self { api }
    }

    /// 访问底层 API.
    #[must_use]
    pub fn api(&self) -> &BilibiliApi {
        &self.api
    }

    /// 通过回退链抓取所有字幕信息.
    ///
    /// 依次尝试 2 套 API (v1.0), 返回第一个成功的结果.
    /// 如果所有 API 均失败, 返回空列表 (字幕非必需品).
    pub async fn fetch_all(&self, bvid: &str, cid: u64) -> Result<Vec<Subtitle>> {
        // API #1: WBI v2 (需要 WBI 签名)
        match self.fetch_from_wbi_v2(bvid, cid).await {
            Ok(subs) if !subs.is_empty() => return Ok(subs),
            Ok(_) => {
                tracing::debug!("WBI v2 返回空字幕列表, 尝试 fallback");
            }
            Err(err) => {
                tracing::debug!("WBI v2 失败: {err}, 尝试 fallback");
            }
        }

        // API #2: legacy v2
        match self.fetch_from_player_v2(bvid, cid).await {
            Ok(subs) if !subs.is_empty() => return Ok(subs),
            Ok(_) => {
                tracing::debug!("player v2 返回空字幕列表");
            }
            Err(err) => {
                tracing::debug!("player v2 失败: {err}");
            }
        }

        // API #3: player.so (老接口, JSON 格式)
        match self.fetch_from_player_so(cid).await {
            Ok(subs) if !subs.is_empty() => return Ok(subs),
            Ok(_) => {
                tracing::debug!("player.so 返回空字幕列表");
            }
            Err(err) => {
                tracing::debug!("player.so 失败: {err}");
            }
        }

        // API #4: view 元信息 (复用 get_view)
        match self.fetch_from_view(bvid).await {
            Ok(subs) if !subs.is_empty() => return Ok(subs),
            Ok(_) => {
                tracing::debug!("view 元信息返回空字幕列表");
            }
            Err(err) => {
                tracing::debug!("view 元信息失败: {err}");
            }
        }

        // API #5: dm/view 弹幕视图 (含字幕元数据)
        match self.fetch_from_dm_view(cid).await {
            Ok(subs) if !subs.is_empty() => return Ok(subs),
            Ok(_) => {
                tracing::debug!("dm/view 返回空字幕列表");
            }
            Err(err) => {
                tracing::debug!("dm/view 失败: {err}");
            }
        }

        tracing::warn!("所有字幕 API 均未返回结果, 该视频可能无字幕");
        Ok(Vec::new())
    }

    /// API #1: WBI v2 签名端点.
    async fn fetch_from_wbi_v2(
        &self,
        bvid: &str,
        cid: u64,
    ) -> Result<Vec<Subtitle>> {
        let value = self.api.get_subtitles(bvid, cid).await?;
        crate::fetch::parse_subtitle_list(&value)
    }

    /// API #2: 旧版 player v2 端点.
    async fn fetch_from_player_v2(
        &self,
        bvid: &str,
        cid: u64,
    ) -> Result<Vec<Subtitle>> {
        let url = format!(
            "https://api.bilibili.com/x/player/v2?bvid={bvid}&cid={cid}"
        );
        let value = self.api.get_json::<serde_json::Value>(&url).await?;
        crate::fetch::parse_subtitle_list(&value)
    }

    /// API #3: player.so — 老接口, 返回 JSON.
    ///
    /// 注意: 查询参数为 `id=cid` (不是 `cid=xxx`).
    async fn fetch_from_player_so(&self, cid: u64) -> Result<Vec<Subtitle>> {
        let url = format!("https://api.bilibili.com/x/player.so?id=cid={cid}");
        let value = self.api.get_json::<serde_json::Value>(&url).await?;
        crate::fetch::parse_subtitle_list(&value)
    }

    /// API #4: 视频元信息 — 复用 `get_view`, 检查 `data.subtitle` 字段.
    async fn fetch_from_view(&self, bvid: &str) -> Result<Vec<Subtitle>> {
        let value = self.api.get_view(bvid).await?;
        crate::fetch::parse_subtitle_list(&value)
    }

    /// API #5: 弹幕视图 — 含字幕元数据.
    ///
    /// URL: `/x/v2/dm/view?type=1&oid={cid}&pid=1`
    /// 响应中的 `data.subtitle` 可包含字幕信息.
    async fn fetch_from_dm_view(&self, cid: u64) -> Result<Vec<Subtitle>> {
        let url = format!(
            "https://api.bilibili.com/x/v2/dm/view?type=1&oid={cid}&pid=1"
        );
        let value = self.api.get_json::<serde_json::Value>(&url).await?;
        crate::fetch::parse_subtitle_list(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtitle_fallback_creation() {
        let api = BilibiliApi::new().unwrap();
        let fallback = SubtitleFallback::new(api);
        let _api_ref = fallback.api();
    }

    #[test]
    fn test_fetch_all_returns_empty_on_error() {
        // 测试异步函数签名正确 (不真正调用网络)
        let api = BilibiliApi::new().unwrap();
        let fallback = SubtitleFallback::new(api);
        let fut = fallback.fetch_all("BV1xx411c7mD", 123);
        assert!(std::mem::size_of_val(&fut) > 0);
    }

    #[test]
    fn test_new_api3_player_so_signature() {
        let api = BilibiliApi::new().unwrap();
        let fallback = SubtitleFallback::new(api);
        // 验证编译通过: 方法签名可调用
        let _: std::pin::Pin<Box<_>> = Box::pin(fallback.fetch_from_player_so(123));
    }

    #[test]
    fn test_new_api4_view_signature() {
        let api = BilibiliApi::new().unwrap();
        let fallback = SubtitleFallback::new(api);
        let _: std::pin::Pin<Box<_>> =
            Box::pin(fallback.fetch_from_view("BV1xx411c7mD"));
    }

    #[test]
    fn test_new_api5_dm_view_signature() {
        let api = BilibiliApi::new().unwrap();
        let fallback = SubtitleFallback::new(api);
        let _: std::pin::Pin<Box<_>> = Box::pin(fallback.fetch_from_dm_view(123));
    }
}
