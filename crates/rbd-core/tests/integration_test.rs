//! 网络集成测试 — 真实 B 站 API 调用.
//!
//! 这些测试需要网络连接。不需要登录的测试直接运行，需要登录的测试标记 #[ignore].
//!
//! 运行:
//! ```bash
//! # 仅运行无需登录的公开 API 测试
//! cargo test -p rbd-core --test integration_test
//!
//! # 运行所有测试 (含需登录的)
//! cargo test -p rbd-core --test integration_test -- --include-ignored
//! ```

use anyhow::Result;
use rbd_core::{
    api::BilibiliApi,
    bv::av_to_bv,
    id::{self, NormalizedId},
};

// --------------------------------------------------------------------------
// 公共 API 测试 (无需登录)
// --------------------------------------------------------------------------

/// B站 API 基础连通性: 验证 `nav` 接口可访问 + WBI key 提取成功.
#[tokio::test]
async fn test_bilibili_api_connectivity() -> Result<()> {
    let api = BilibiliApi::new()?;
    let key = api.refresh_wbi_key().await?;
    assert!(!key.img_key.is_empty(), "img_key 不应为空");
    assert!(!key.sub_key.is_empty(), "sub_key 不应为空");
    Ok(())
}

/// 通过公开 API 获取视频基本信息 (view).
#[tokio::test]
async fn test_get_view_public() -> Result<()> {
    let api = BilibiliApi::new()?;
    let resp: serde_json::Value = api.get_view("BV1GJ411x7h7").await?;

    let data = &resp["data"];
    assert!(
        data["bvid"].as_str().unwrap_or("") == "BV1GJ411x7h7",
        "bvid 不匹配"
    );

    // 标题和 UP 主名应存在
    let title = data["title"].as_str().unwrap_or("");
    assert!(!title.is_empty(), "标题不应为空");

    let owner = &data["owner"];
    assert!(
        !owner["name"].as_str().unwrap_or("").is_empty(),
        "UP 主名不应为空"
    );

    Ok(())
}

/// 通过公开 API 获取分 P 列表 (pagelist).
#[tokio::test]
async fn test_get_pagelist_public() -> Result<()> {
    let api = BilibiliApi::new()?;
    let resp: serde_json::Value = api.get_pagelist("BV17x411w7KC").await?;

    let pages = resp["data"].as_array().expect("data 应为数组");
    assert!(pages.len() >= 2, "该视频至少有 2 个分P"); // BV17x411w7KC 已知 10P

    // 每个分P 应有 cid 和 part 字段
    for page in pages {
        assert!(page["cid"].as_u64().is_some(), "分P 缺少 cid");
        assert!(!page["part"].as_str().unwrap_or("").is_empty(), "分P 缺少 part 名称");
    }

    Ok(())
}

/// URL 解析 → BV → API 完整链路.
#[tokio::test]
async fn test_url_parse_to_api_roundtrip() -> Result<()> {
    // 解析 URL
    let result = id::parse_url("https://www.bilibili.com/video/BV1GJ411x7h7")?;

    // 提取 bvid
    let bvid = match &result {
        NormalizedId::UgcVideo { id, .. } => id.clone(),
        _ => anyhow::bail!("解析为 {:?}", result),
    };

    // 调用 API 验证
    let api = BilibiliApi::new()?;
    let resp: serde_json::Value = api.get_view(&bvid).await?;

    assert_eq!(
        resp["data"]["bvid"].as_str().unwrap_or(""),
        "BV1GJ411x7h7"
    );

    Ok(())
}

/// 测试多种 URL 格式解析 (不调用 API).
#[test]
fn test_url_parsing_table() -> Result<()> {
    let cases = vec![
        ("https://www.bilibili.com/video/BV1GJ411x7h7", "UGC 视频"),
        ("https://b23.tv/av170001", "短链"),
        ("https://www.bilibili.com/bangumi/play/ss39443", "番剧"),
        ("https://www.bilibili.com/bangumi/play/ep250712", "番剧单集"),
        ("https://space.bilibili.com/477332594", "UP 主空间"),
        ("BV1xx411c7mD", "UGC 视频"),
    ];

    for (url, expected_prefix) in cases {
        let result = id::parse_url(url);
        match result {
            Ok(id) => {
                let display = id.to_string();
                assert!(
                    display.starts_with(expected_prefix),
                    "URL '{url}': 期望以 '{expected_prefix}' 开头, 得到 '{display}'"
                );
            }
            Err(e) => {
                // 有些 URL 可能因网络原因失败 (如短链接需 resolve)，但不应是解析错误
                anyhow::bail!("URL '{url}' 解析失败: {e}");
            }
        }
    }

    Ok(())
}

/// BV↔AV 转换一致性检查.
#[test]
fn test_bv_av_conversion_consistency() -> Result<()> {
    // 已知的 BV/AV 对应关系 (来自 BBDown 参考实现)
    let cases: Vec<(u64, &str)> = vec![
        (170001, "BV17x411w7KC"),
        (4567890, "BV1gs411B7Mu"),
    ];

    for (av, expected_bv) in cases {
        let bv = av_to_bv(av)?;
        assert_eq!(
            bv, expected_bv,
            "av{av} -> {bv}, 期望 {expected_bv}"
        );
    }

    Ok(())
}

// --------------------------------------------------------------------------
// 需登录的 API 测试 (#[ignore])
// --------------------------------------------------------------------------

/// 登录后获取 playurl (需要 SESSDATA).
/// 设置环境变量 `BILI_SESSDATA` 后运行:
/// ```bash
/// $env:BILI_SESSDATA='your_sessdata'; cargo test -p rbd-core --test integration_test -- test_playurl_with_auth --include-ignored
/// ```
#[tokio::test]
#[ignore = "需要 SESSDATA — 设置 BILI_SESSDATA 环境变量后运行"]
async fn test_playurl_with_auth() -> Result<()> {
    let sessdata = std::env::var("BILI_SESSDATA").ok();
    let sessdata = sessdata.as_deref().unwrap_or("");
    if sessdata.is_empty() {
        anyhow::bail!("BILI_SESSDATA 未设置");
    }

    let base = BilibiliApi::new()?;
    let api = base.with_cookie(sessdata)?;

    // 先获取视频 info 拿 cid
    let view: serde_json::Value = api.get_view("BV1GJ411x7h7").await?;
    let cid = view["data"]["cid"]
        .as_u64()
        .unwrap_or_else(|| view["data"]["pages"][0]["cid"].as_u64().unwrap_or(0));
    assert_ne!(cid, 0, "cid 不应为 0");

    // 获取 playurl (高质量需要登录)
    let playurl: serde_json::Value = api.get_playurl("BV1GJ411x7h7", cid, 127).await?;

    // DASH 响应应有 dash 字段
    let dash = &playurl["data"]["dash"];
    if dash.is_object() {
        let videos = dash["video"].as_array();
        let audios = dash["audio"].as_array();
        eprintln!(
            "DASH: {} video tracks, {} audio tracks",
            videos.map_or(0, |v| v.len()),
            audios.map_or(0, |a| a.len())
        );
    }

    Ok(())
}

/// 登录后获取字幕 (需要 SESSDATA + WBI).
#[tokio::test]
#[ignore = "需要 SESSDATA — 设置 BILI_SESSDATA 环境变量后运行"]
async fn test_subtitles_with_auth() -> Result<()> {
    let sessdata = std::env::var("BILI_SESSDATA").ok();
    let sessdata = sessdata.as_deref().unwrap_or("");
    if sessdata.is_empty() {
        anyhow::bail!("BILI_SESSDATA 未设置");
    }

    let base = BilibiliApi::new()?;
    let api = base.with_cookie(sessdata)?;

    // 一个有字幕的视频
    let subtitles: serde_json::Value = api.get_subtitles("BV1GJ411x7h7", 171370127).await?;

    let subs = &subtitles["data"]["subtitle"]["subtitles"];
    if subs.is_array() {
        eprintln!("字幕: {} 个", subs.as_array().unwrap().len());
    }

    Ok(())
}
