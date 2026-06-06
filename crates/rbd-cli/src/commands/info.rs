//! rbd info — 解析 URL 并显示视频信息 (不下载).
use anyhow::Result;
use rbd_core::api::BilibiliApi;
use rbd_core::extractor::ExtractorRegistry;
use rbd_core::id::parse_url;

/// 仅解析 URL 并显示信息, 不执行下载.
pub async fn run(url: &str) -> Result<()> {
    let normalized = parse_url(url)?;
    println!("URL:        {url}");
    println!("类型:        {normalized}");

    let api = BilibiliApi::new()?;
    let registry = ExtractorRegistry::with_defaults();

    match registry.extract(&normalized, &api).await {
        Ok(vinfo) => {
            println!("标题:        {}", vinfo.title);
            if !vinfo.owner_name.is_empty() {
                println!(
                    "UP主:        {} (mid={})",
                    vinfo.owner_name, vinfo.owner_mid
                );
            }
            println!("分P数:       {}", vinfo.pages.len());
            for (i, p) in vinfo.pages.iter().enumerate() {
                println!("  P{}: {} ({}s)", i + 1, p.title, p.duration);
            }
            if vinfo.is_bangumi {
                println!("类型:        番剧");
            }
            if vinfo.is_cheese {
                println!("类型:        课程");
            }
            if !vinfo.tags.is_empty() {
                println!("标签:        {}", vinfo.tags.join(", "));
            }
        }
        Err(e) => {
            tracing::warn!("提取失败: {e}");
            println!("无法提取视频信息: {e}");
        }
    }
    Ok(())
}
