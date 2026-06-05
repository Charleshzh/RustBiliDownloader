//! rbd download — 完整下载管线.
//!
//! URL 解析 → 视频信息提取 → playurl 获取 → 下载 → 混流.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
use std::path::{Path, PathBuf};

use rbd_core::api::BilibiliApi;
use rbd_core::extractor::ExtractorRegistry;
use rbd_core::id::parse_url;
use rbd_core::model::{AudioTrack, VideoTrack};
use rbd_downloader::event::DownloadEvent;
use rbd_downloader::manager::{DownloadManager, DownloadMode};
use rbd_downloader::parallel::DownloadSpec;
use rbd_playurl::client::PlayUrlClient;
use rbd_playurl::fallback::FallbackChain;

use crate::progress::CliProgress;

/// 下载命令参数.
#[derive(Debug, Clone)]
pub struct DownloadArgs {
    /// 视频 URL 或 BV/AV 号
    pub url: String,
    /// 输出目录
    pub output_dir: Option<PathBuf>,
    /// 画质代码 (默认 80=1080P)
    pub quality: Option<u32>,
    /// 视频编码优先级 (avc / hevc / av1)
    pub vcodec_priority: Option<String>,
    /// 多线程数 (默认 8)
    pub num_workers: Option<u32>,
    /// 仅下载视频
    pub video_only: bool,
    /// 仅下载音频
    pub audio_only: bool,
    /// 不下载弹幕
    pub no_danmaku: bool,
    /// 不下载字幕
    pub no_subtitle: bool,
    /// 不下载封面
    pub no_cover: bool,
    /// 交互式选 track
    pub interactive: bool,
    /// 使用 aria2c
    pub aria2c: bool,
}

/// 执行下载命令.
pub async fn run(args: DownloadArgs) -> Result<()> {
    // 1. 解析 URL
    let normalized = parse_url(&args.url)
        .with_context(|| format!("URL 解析失败: {}", args.url))?;
    tracing::info!("已解析: {:?}", normalized);

    // 2. 构建 API 客户端
    let api = BilibiliApi::new()?;

    // 3. 提取视频信息
    let registry = ExtractorRegistry::with_defaults();
    let vinfo = registry
        .extract(&normalized, &api)
        .await
        .with_context(|| "提取视频信息失败")?;
    tracing::info!("标题: {}", vinfo.title);

    // 4. 确定输出目录
    let output_dir = args
        .output_dir
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    std::fs::create_dir_all(&output_dir)?;

    // 5. 应用 quality / no_cover / interactive 参数
    let min_quality = args.quality.unwrap_or(80);
    let is_interactive = args.interactive;

    // 下载封面 (除非 --no-cover)
    if !args.no_cover && !vinfo.pic.is_empty() {
        download_cover(&api, &vinfo.pic, &output_dir, &vinfo.title).await?;
    }

    // 6. 逐页下载
    let is_bangumi = vinfo.is_bangumi;

    for (i, page) in vinfo.pages.iter().enumerate() {
        if vinfo.pages.len() > 1 {
            tracing::info!(
                "下载分 P {}/{}: {}",
                i + 1,
                vinfo.pages.len(),
                page.title
            );
        }

        // 获取 playurl
        // 获取 playurl (复用 API 客户端, clone 共享连接池)
        let play_client = PlayUrlClient::new(api.clone());
        let chain = FallbackChain::default_for(None, is_bangumi)
            .with_min_quality(min_quality);

        let fetch_result = chain
            .fetch(&play_client, &vinfo.bvid, page.cid, is_bangumi)
            .await
            .with_context(|| format!("获取 playurl 失败 (分 P {})", i + 1))?;

        // 选择最佳视频/音频 (或交互式选择)
        let (video, audio) = if is_interactive {
            select_tracks_interactive(&fetch_result.videos, &fetch_result.audios, args.video_only, args.audio_only)
        } else {
            let v = if !args.audio_only {
                select_best_video(&fetch_result.videos, args.vcodec_priority.as_deref())
            } else {
                None
            };
            let a = if !args.video_only {
                select_best_audio(&fetch_result.audios)
            } else {
                None
            };
            (v, a)
        };

        // 构建目标路径
        let safe_title = rbd_foundation::path::sanitize_filename(&format!(
            "{}_{}_{}",
            vinfo.title,
            page.page_index,
            i + 1
        ));
        // 防止路径穿越 (跨平台: 同时处理 / 和 \)
        let safe_title: String = safe_title
            .replace('\\', "/")
            .split('/')
            .filter(|s| *s != ".." && !s.is_empty())
            .collect::<Vec<_>>()
            .join("_");

        let is_combined = video.map_or(false, |v| v.is_combined);
        let video_ext = if is_combined { "mp4" } else { "m4s" };
        let video_path = video.map(|_| output_dir.join(format!("{safe_title}_video.{video_ext}")));
        let audio_path = audio.map(|_| output_dir.join(format!("{safe_title}_audio.m4s")));

        // 下载
        let mode = if args.aria2c {
            DownloadMode::Aria2c
        } else {
            DownloadMode::Parallel
        };
        let manager = DownloadManager::new(mode);

        let headers = build_default_headers();

        let video_spec = video.and_then(|v| {
            let dest = video_path.clone()?;
            let url = v.urls.first()?.clone();
            if !rbd_core::is_safe_download_url(&url) {
                tracing::warn!("跳过不安全的下载 URL: {url}");
                return None;
            }
            Some(DownloadSpec {
                url,
                headers: headers.clone(),
                dest,
                task_id: format!("video-{}", v.id),
                num_threads: args.num_workers.unwrap_or(8) as usize,
                block_size: 1024 * 1024,
            })
        });

        let audio_spec = audio.and_then(|a| {
            let dest = audio_path.clone()?;
            let url = a.urls.first()?.clone();
            if !rbd_core::is_safe_download_url(&url) {
                tracing::warn!("跳过不安全的下载 URL: {url}");
                return None;
            }
            Some(DownloadSpec {
                url,
                headers,
                dest,
                task_id: format!("audio-{}", a.id),
                num_threads: 4,
                block_size: 1024 * 1024,
            })
        });

        // 创建进度条并在下载回调中实时更新
        let progress_bar = CliProgress::new(100, &format!("下载 {}", safe_title));
        let pb = progress_bar.clone();

        let (v_path_opt, a_path_opt) = manager
            .download_concurrent(
                video_spec,
                audio_spec,
                None,
                move |event| {
                    match event {
                        DownloadEvent::Start { task_id, total } => {
                            tracing::info!("开始下载: {task_id} ({total} 字节)");
                        }
                        DownloadEvent::Progress { downloaded, total, task_id, .. } => {
                            let pct = if total > 0 { (downloaded * 100 / total) as u64 } else { 0 };
                            pb.set_position(pct);
                            tracing::debug!("{task_id}: {downloaded}/{total}");
                        }
                        DownloadEvent::Done { task_id, .. } => {
                            tracing::info!("下载完成: {task_id}");
                        }
                        _ => {}
                    }
                },
            )
            .await
            .with_context(|| format!("下载失败 (分 P {})", i + 1))?;

        progress_bar.finish_with_message(&format!("分 P {} 下载完成", i + 1));

        // 混流: 仅当视频和音频都存在时执行
        match (v_path_opt.as_ref(), a_path_opt.as_ref()) {
            (Some(v_path), Some(a_path)) => {
                let video_codec = video.map(|v| v.codec.as_str()).unwrap_or("avc");
                let audio_codec = audio.map(|a| a.codec.as_str()).unwrap_or("aac");
                let is_hdr = video.map_or(false, |v| v.is_hdr);
                let is_dolby = video.map_or(false, |v| v.is_dolby_vision);
                mux_files(
                    v_path,
                    a_path,
                    &output_dir,
                    &safe_title,
                    video_codec,
                    audio_codec,
                    is_hdr,
                    is_dolby,
                )
                .with_context(|| "混流失败")?;
            }
            (Some(v_path), None) => {
                if is_combined {
                    tracing::info!("下载完成 (含音视频编码流): {}", v_path.display());
                } else {
                    tracing::info!("仅下载视频: {}", v_path.display());
                }
            }
            (None, Some(a_path)) => {
                tracing::info!("仅下载音频: {}", a_path.display());
            }
            (None, None) => {
                tracing::warn!("无可用流下载");
            }
        }

        // 下载字幕
        if !args.no_subtitle {
            match fetch_subtitles(
                &api,
                &vinfo.bvid,
                page.cid,
                &output_dir,
                &safe_title,
            )
            .await
            {
                Ok(n) if n > 0 => tracing::info!("下载了 {n} 个字幕"),
                Ok(_) => tracing::debug!("无字幕"),
                Err(e) => tracing::warn!("字幕下载失败: {e}"),
            }
        }

        // 下载弹幕
        if !args.no_danmaku {
            match fetch_danmaku(&api, &vinfo.bvid, page.cid, &output_dir, &safe_title)
            {
                Ok(()) => tracing::info!("弹幕已下载"),
                Err(e) => tracing::warn!("弹幕下载失败: {e}"),
            }
        }

        tracing::info!("分 P {} 完成", i + 1);
    }

    tracing::info!("全部下载完成");
    Ok(())
}

/// 选择最佳视频轨.
fn select_best_video<'a>(
    tracks: &'a [VideoTrack],
    priority_codec: Option<&str>,
) -> Option<&'a VideoTrack> {
    if tracks.is_empty() {
        return None;
    }
    let mut sorted: Vec<&VideoTrack> = tracks.iter().collect();
    sorted.sort_by(|a, b| b.quality.cmp(&a.quality));

    // 如果有编码优先级, 优先选择匹配的
    if let Some(prio) = priority_codec {
        let prio_lower = prio.to_lowercase();
        for t in &sorted {
            if t.codec.to_lowercase() == prio_lower {
                return Some(t);
            }
        }
    }
    sorted.into_iter().next()
}

/// 选择最佳音频轨.
fn select_best_audio<'a>(tracks: &'a [AudioTrack]) -> Option<&'a AudioTrack> {
    if tracks.is_empty() {
        return None;
    }
    let mut sorted: Vec<&AudioTrack> = tracks.iter().collect();
    // 优先 Hi-Res, 其次杜比, 再按质量排序
    sorted.sort_by(|a, b| {
        b.is_hi_res
            .cmp(&a.is_hi_res)
            .then_with(|| b.is_dolby_atmos.cmp(&a.is_dolby_atmos))
            .then_with(|| b.quality.cmp(&a.quality))
    });
    sorted.into_iter().next()
}

/// 构建默认请求头.
fn build_default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        ),
    );
    headers.insert(REFERER, HeaderValue::from_static("https://www.bilibili.com"));
    headers
}

/// 混流 video + audio → MP4.
///
/// 根据实际的视频/音频编码选择合适的混流策略:
/// - `avc` + `aac`: DashCopy (已委托 ffmpeg)
/// - `hevc` / `av1` / 杜比: ffmpeg merge/transcode
fn mux_files(
    video_path: &Path,
    audio_path: &Path,
    output_dir: &Path,
    safe_title: &str,
    video_codec: &str,
    audio_codec: &str,
    is_hdr: bool,
    is_dolby_vision: bool,
) -> Result<()> {
    use rbd_muxer::strategy::{choose_strategy, MuxStrategy};

    let output_path = output_dir.join(format!("{safe_title}.mp4"));

    let strategy = choose_strategy(video_codec, audio_codec, is_hdr, is_dolby_vision);

    match strategy {
        MuxStrategy::DashCopy => {
            let muxer = rbd_muxer::dash_copy::DashCopyMuxer::new();
            muxer.mux(
                &[video_path.to_path_buf()],
                Some(&[audio_path.to_path_buf()]),
                &output_path,
            )?;
            tracing::info!("混流完成: {}", output_path.display());
        }
        MuxStrategy::FfmpegMerge => {
            let muxer = rbd_muxer::ffmpeg::FfmpegMuxer::new()?;
            muxer.merge_copy(video_path, Some(audio_path), &output_path)?;
            tracing::info!("ffmpeg 混流完成: {}", output_path.display());
        }
        MuxStrategy::FfmpegTranscode => {
            let muxer = rbd_muxer::ffmpeg::FfmpegMuxer::new()?;
            // 根据目标 codec 选择编码器
            let target_vcodec = match video_codec {
                "hevc" | "dvh1" | "dvhe" => "libx265",
                "av1" => "libaom-av1",
                _ => "libx264",
            };
            let target_acodec = match audio_codec {
                "flac" | "eac3" => "aac",
                _ => audio_codec,
            };
            muxer.transcode(video_path, Some(audio_path), &output_path, target_vcodec, target_acodec)?;
            tracing::info!("ffmpeg 转码完成: {}", output_path.display());
        }
    }
    Ok(())
}

/// 下载字幕.
async fn fetch_subtitles(
    api: &BilibiliApi,
    bvid: &str,
    cid: u64,
    output_dir: &Path,
    title: &str,
) -> Result<usize> {
    // BilibiliApi 不可 Clone, 需创建新实例
    let api2 = BilibiliApi::new()?;
    let _sf = rbd_subtitle::fallback::SubtitleFallback::new(api2);

    // 直接使用 BilibiliApi 获取字幕列表
    let subtitle_list = rbd_subtitle::fetch::fetch_subtitle_list(api, bvid, cid).await?;

    let mut count = 0;
    for sub in subtitle_list {
        match rbd_subtitle::fetch::fetch_subtitle_content(&sub.url).await {
            Ok((format, content)) => {
                let ext = match format {
                    rbd_subtitle::model::SubtitleFormat::Srt => "srt",
                    rbd_subtitle::model::SubtitleFormat::Ass => "ass",
                    rbd_subtitle::model::SubtitleFormat::Json => "json",
                };
                let path = output_dir.join(format!("{title}_{}.{ext}", sub.lang));
                std::fs::write(&path, content)?;
                count += 1;
            }
            Err(e) => {
                tracing::warn!("字幕 {} 下载失败: {e}", sub.lang);
            }
        }
    }
    Ok(count)
}

/// 下载弹幕.
fn fetch_danmaku(
    _api: &BilibiliApi,
    _bvid: &str,
    _cid: u64,
    _output_dir: &Path,
    _title: &str,
) -> Result<()> {
    // TODO M5: 从 B 站 API 获取弹幕并渲染为 ASS
    tracing::debug!("弹幕下载功能将在 M5 实现");
    Ok(())
}

/// 交互式选择视频/音频轨.
fn select_tracks_interactive<'a>(
    videos: &'a [VideoTrack],
    audios: &'a [AudioTrack],
    video_only: bool,
    audio_only: bool,
) -> (Option<&'a VideoTrack>, Option<&'a AudioTrack>) {
    let video = if !audio_only && !videos.is_empty() {
        let items: Vec<String> = videos
            .iter()
            .map(|v| format!("{} ({}) {}fps {:>6}kbps", v.quality_desc, v.codec, v.frame_rate, v.bandwidth / 1000))
            .collect();
        match dialoguer::Select::new()
            .with_prompt("选择视频画质")
            .items(&items)
            .default(0)
            .interact()
        {
            Ok(idx) => videos.get(idx),
            Err(_) => Some(&videos[0]),
        }
    } else {
        None
    };

    let audio = if !video_only && !audios.is_empty() {
        let items: Vec<String> = audios
            .iter()
            .map(|a| format!("{} ({}) {:>6}kbps", a.quality_desc, a.codec, a.bandwidth / 1000))
            .collect();
        match dialoguer::Select::new()
            .with_prompt("选择音频品质")
            .items(&items)
            .default(0)
            .interact()
        {
            Ok(idx) => audios.get(idx),
            Err(_) => Some(&audios[0]),
        }
    } else {
        None
    };

    (video, audio)
}

/// 下载封面图片.
async fn download_cover(_api: &BilibiliApi, pic_url: &str, output_dir: &Path, title: &str) -> Result<()> {
    let safe_title = rbd_foundation::path::sanitize_filename(title);
    let ext = pic_url
        .rsplit('.')
        .next()
        .filter(|s| s.len() <= 5)
        .unwrap_or("jpg");
    let cover_path = output_dir.join(format!("{safe_title}_cover.{ext}"));

    if cover_path.exists() {
        tracing::debug!("封面已存在: {}", cover_path.display());
        return Ok(());
    }

    let bytes = reqwest::get(pic_url)
        .await
        .with_context(|| format!("封面下载失败: {pic_url}"))?
        .bytes()
        .await?;

    std::fs::write(&cover_path, &bytes)?;
    tracing::info!("封面已保存: {}", cover_path.display());
    Ok(())
}
