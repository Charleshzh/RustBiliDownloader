//! playurl fallback 链.

use anyhow::{anyhow, Result};
use rbd_core::{AudioTrack, SubtitleTrack, VideoTrack};

use crate::{app_grpc, client::PlayUrlClient, html5, tv, web, web_bangumi};

/// playurl 模式.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// HTML5 最低优先级.
    Html5,
    /// Web 主模式.
    Web,
    /// 番剧 Web 模式.
    WebBangumi,
    /// TV 模式.
    Tv,
    /// APP 模式.
    App,
}

impl Mode {
    /// 所有模式.
    pub const ALL: [Mode; 5] = [
        Mode::Html5,
        Mode::Web,
        Mode::WebBangumi,
        Mode::Tv,
        Mode::App,
    ];

    /// 模式名称.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Html5 => "html5",
            Self::Web => "web",
            Self::WebBangumi => "web_bangumi",
            Self::Tv => "tv",
            Self::App => "app",
        }
    }

    /// 是否仅适用于番剧.
    #[must_use]
    pub fn is_for_bangumi(&self) -> bool {
        matches!(self, Self::WebBangumi)
    }
}

/// fallback 命中结果.
#[derive(Debug, Clone)]
pub struct FetchResult {
    /// 命中的模式.
    pub mode: Mode,
    /// 视频轨.
    pub videos: Vec<VideoTrack>,
    /// 音频轨.
    pub audios: Vec<AudioTrack>,
    /// 字幕轨.
    pub subtitles: Vec<SubtitleTrack>,
    /// 命中画质描述.
    pub quality_desc: String,
}

/// fallback 链配置.
#[derive(Debug, Clone)]
pub struct FallbackChain {
    /// 模式列表.
    pub modes: Vec<Mode>,
    /// 画质优先级 (从高到低).
    pub quality_priority: Vec<u32>,
    /// 最低可接受画质 (低于此值的 qn 将被跳过).
    pub min_quality: Option<u32>,
}

impl FallbackChain {
    /// 默认 fallback 配置.
    #[must_use]
    pub fn default_for(_uid: Option<u64>, is_bangumi: bool) -> Self {
        let modes = if is_bangumi {
            vec![Mode::WebBangumi, Mode::Web, Mode::Tv]
        } else {
            vec![Mode::Web, Mode::Tv, Mode::Html5]
        };
        Self {
            modes,
            quality_priority: vec![127, 126, 125, 120, 116, 112, 80, 74, 64, 32, 16],
            min_quality: None,
        }
    }

    /// 设置最低可接受画质.
    #[must_use]
    pub fn with_min_quality(mut self, qn: u32) -> Self {
        self.min_quality = Some(qn);
        self
    }

    /// 按模式与画质顺序拉取轨道.
    pub async fn fetch(
        &self,
        client: &PlayUrlClient,
        bvid: &str,
        cid: u64,
        is_bangumi: bool,
    ) -> Result<FetchResult> {
        let mut last_error = None;

        // 外层 mode(保证 Web/Tv 优先于 Html5), 内层 qn(同一模式内画质从高到低)
        for &mode in &self.modes {
            if is_bangumi && mode == Mode::Html5 {
                continue;
            }
            if !is_bangumi && mode.is_for_bangumi() {
                continue;
            }

            for &qn in &self.quality_priority {
                if let Some(min) = self.min_quality {
                    if qn < min {
                        continue;
                    }
                }

                match fetch_mode(mode, client, bvid, cid, qn).await {
                    Ok((videos, audios)) => {
                        if !videos.is_empty() || !audios.is_empty() {
                            let subtitles =
                                client.fetch_subtitles(bvid, cid).await.unwrap_or_default();
                            let quality_desc = videos
                                .first()
                                .map(|item| item.quality_desc.clone())
                                .or_else(|| audios.first().map(|item| item.quality_desc.clone()))
                                .unwrap_or_else(|| format!("Q{qn}"));
                            return Ok(FetchResult {
                                mode,
                                videos,
                                audios,
                                subtitles,
                                quality_desc,
                            });
                        }
                        last_error = Some(anyhow!("模式 {} Q{qn} 返回空轨道", mode.name()));
                    }
                    Err(err) => last_error = Some(err),
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("所有 playurl 模式均失败")))
    }
}

async fn fetch_mode(
    mode: Mode,
    client: &PlayUrlClient,
    bvid: &str,
    cid: u64,
    qn: u32,
) -> Result<(Vec<VideoTrack>, Vec<AudioTrack>)> {
    match mode {
        Mode::Html5 => html5::fetch_html5(client, bvid, cid, qn).await,
        Mode::Web => web::fetch_web(client, bvid, cid, qn).await,
        Mode::WebBangumi => web_bangumi::fetch_web_bangumi(client, bvid, cid, qn).await,
        Mode::Tv => tv::fetch_tv(client, bvid, cid, qn).await,
        Mode::App => app_grpc::fetch_app_grpc(client, bvid, cid, qn).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_name() {
        assert_eq!(Mode::Html5.name(), "html5");
        assert_eq!(Mode::Web.name(), "web");
        assert_eq!(Mode::WebBangumi.name(), "web_bangumi");
        assert_eq!(Mode::Tv.name(), "tv");
        assert_eq!(Mode::App.name(), "app");
    }

    #[test]
    fn test_mode_is_for_bangumi() {
        assert!(Mode::WebBangumi.is_for_bangumi());
        assert!(!Mode::Web.is_for_bangumi());
    }

    #[test]
    fn test_default_chain_for_bangumi() {
        let chain = FallbackChain::default_for(None, true);
        assert_eq!(chain.modes.first(), Some(&Mode::WebBangumi));
    }

    #[test]
    fn test_default_chain_for_normal() {
        let chain = FallbackChain::default_for(None, false);
        assert_eq!(chain.modes.first(), Some(&Mode::Web));
    }

    #[test]
    fn test_fallback_chain_iterates_modes() {
        let chain = FallbackChain::default_for(None, false);
        assert!(chain.modes.len() >= 3);
    }
}
