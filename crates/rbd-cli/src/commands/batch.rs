//! rbd batch — 批量下载.
//! 每行一个 URL, # 开头为注释, 空行跳过.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// 执行批量下载命令.
pub async fn run(file: PathBuf) -> Result<()> {
    let content = fs::read_to_string(&file)
        .with_context(|| format!("读取文件失败: {}", file.display()))?;

    let urls: Vec<&str> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    if urls.is_empty() {
        anyhow::bail!("文件中无有效 URL");
    }

    tracing::info!("批量下载 {} 个视频", urls.len());

    for (i, url) in urls.iter().enumerate() {
        tracing::info!("[{}/{}] {}", i + 1, urls.len(), url);
        super::download::run(super::download::DownloadArgs {
            url: (*url).to_string(),
            output_dir: None,
            quality: None,
            vcodec_priority: None,
            num_workers: None,
            video_only: false,
            audio_only: false,
            no_danmaku: false,
            no_subtitle: false,
            no_cover: false,
            interactive: false,
            aria2c: false,
        })
        .await
        .with_context(|| format!("下载失败 [{}]: {}", i + 1, url))?;
    }

    tracing::info!("批量下载全部完成");
    Ok(())
}
