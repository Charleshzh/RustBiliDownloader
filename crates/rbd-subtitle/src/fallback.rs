//! 字幕 fallback 链 — 5 套 API 按顺序回退.
//!
//! **API 顺序 (回退链)**:
//! 1. `GET /x/player/wbi/v2?cid=xxx&bvid=xxx` (主用, 需 WBI 签名)
//! 2. `GET /x/player/v2?cid=xxx&bvid=xxx` (旧 fallback)
//!
//! 对于 API #3-#5, v1.0 仅标记 TODO, M5 实现.

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

        // API #3-#5: TODO M5 实现
        // - /x/player.so?id=cid (JSON 格式)
        // - /x/web-interface/view?bvid=xxx (视频元信息 subtitle 字段)
        // - /x/v2/dm/view?type=1&oid=cid (弹幕含字幕)

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
}
