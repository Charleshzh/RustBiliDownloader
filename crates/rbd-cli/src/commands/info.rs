//! rbd info — 解析 URL 并显示视频信息 (不下载).
use anyhow::{Context, Result};
use rbd_core::api::BilibiliApi;
use rbd_core::extractor::ExtractorRegistry;
use rbd_core::id::parse_url;

/// 仅解析 URL 并显示信息, 不执行下载.
pub async fn run(url: &str) -> Result<()> {
    let normalized = parse_url(url)?;
    println!("URL:        {url}");
    println!("类型:        {normalized}");

    // 加载默认 auth profile, 空间/收藏夹 API 需要 SESSDATA
    let api = load_api_with_auth().await?;
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

/// 加载默认 auth profile 并创建带 cookie 的 API 客户端.
async fn load_api_with_auth() -> Result<BilibiliApi> {
    // 尝试从 keychain 加载 profile
    let profile = rbd_auth::keyring_store::load("default")
        .or_else(|_| rbd_auth::keyring_store::load("ci-test"));
    match profile {
        Ok(p) if p.is_logged_in() => {
            // 使用完整的 Cookie header (含 buvid3/buvid4 等)
            // 而非仅 SESSDATA, 因为空间 API 需要完整浏览器指纹
            let api = BilibiliApi::new()?;
            let cookie_str = p.cookie_header();
            api.with_full_cookie(&cookie_str)
                .context("无法设置 auth cookie")
        }
        _ => {
            tracing::debug!("未找到有效登录态, 使用匿名 API");
            BilibiliApi::new()
        }
    }
}
