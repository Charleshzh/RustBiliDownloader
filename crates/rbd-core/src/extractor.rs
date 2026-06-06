//! Extractor trait + 注册表 + 各类 ID 抽取器.

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::api::BilibiliApi;
use crate::id::NormalizedId;
use crate::model::{Page, VInfo};

/// 抽取器统一接口.
#[async_trait]
pub trait Extractor: Send + Sync {
    /// 名称.
    fn name(&self) -> &'static str;
    /// 是否匹配指定 ID.
    fn matches(&self, id: &NormalizedId) -> bool;
    /// 执行抽取.
    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo>;
}

/// 抽取器注册表.
pub struct ExtractorRegistry {
    fetchers: Vec<Box<dyn Extractor>>,
}

impl ExtractorRegistry {
    /// 创建空注册表.
    pub fn new() -> Self {
        Self { fetchers: vec![] }
    }

    /// 创建带默认 9 个抽取器的注册表.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(NormalExtractor));
        registry.register(Box::new(BangumiExtractor));
        registry.register(Box::new(CheeseExtractor));
        registry.register(Box::new(FavListExtractor));
        registry.register(Box::new(MediaListExtractor));
        registry.register(Box::new(SeriesExtractor));
        registry.register(Box::new(CollectionExtractor));
        registry.register(Box::new(SpaceExtractor));
        registry.register(Box::new(IntlBangumiExtractor));
        registry
    }

    /// 注册抽取器.
    pub fn register(&mut self, e: Box<dyn Extractor>) {
        self.fetchers.push(e);
    }

    /// 查找匹配的抽取器.
    pub fn find(&self, id: &NormalizedId) -> Option<&dyn Extractor> {
        self.fetchers
            .iter()
            .find_map(|f| f.matches(id).then_some(f.as_ref()))
    }

    /// 执行抽取.
    pub async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        match self.find(id) {
            Some(extractor) => extractor.extract(id, api).await,
            None => anyhow::bail!("no extractor matches {id:?}"),
        }
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 普通视频抽取器.
pub struct NormalExtractor;
/// 番剧抽取器.
pub struct BangumiExtractor;
/// 课程抽取器.
pub struct CheeseExtractor;
/// 收藏夹抽取器.
pub struct FavListExtractor;
/// 媒体列表抽取器.
pub struct MediaListExtractor;
/// 合集抽取器.
pub struct SeriesExtractor;
/// 合集 (UP 主合集) 抽取器.
pub struct CollectionExtractor;
/// 空间抽取器.
pub struct SpaceExtractor;
/// 国际版番剧抽取器.
pub struct IntlBangumiExtractor;

#[async_trait]
impl Extractor for NormalExtractor {
    fn name(&self) -> &'static str {
        "normal"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::UgcVideo { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let NormalizedId::UgcVideo { id, .. } = id else {
            anyhow::bail!("normal extractor only supports ugc video")
        };

        let bvid = if id.starts_with("av") {
            crate::bv::av_to_bv(id.trim_start_matches("av").parse()?)?
        } else {
            id.clone()
        };

        let view = api.get_view(&bvid).await?;
        let pagelist = api.get_pagelist(&bvid).await?;

        let mut info = parse_view_response(&view)?;
        let pages = parse_pagelist_response(&pagelist)?;
        if !pages.is_empty() {
            info.cids = pages.iter().map(|page| page.cid).collect();
            info.part_names = pages.iter().map(|page| page.title.clone()).collect();
            info.pages = pages;
        }
        Ok(info)
    }
}

#[async_trait]
impl Extractor for BangumiExtractor {
    fn name(&self) -> &'static str {
        "bangumi"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::Bangumi { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let NormalizedId::Bangumi {
            season_id,
            ep_id,
            media_id,
        } = id
        else {
            anyhow::bail!("bangumi extractor only supports bangumi ids")
        };

        let value = if let Some(season_id) = season_id {
            api.get_bangumi_season(*season_id).await?
        } else if let Some(ep_id) = ep_id {
            api.get_bangumi_ep(*ep_id).await?
        } else if media_id.is_some() {
            anyhow::bail!("media_id based bangumi extraction not yet implemented")
        } else {
            anyhow::bail!("missing bangumi identifier")
        };

        parse_bangumi_season_response(&value)
    }
}

// ── CheeseExtractor ──────────────────────────────────────────────

#[async_trait]
impl Extractor for CheeseExtractor {
    fn name(&self) -> &'static str {
        "cheese"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::Cheese { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let NormalizedId::Cheese { season_id, .. } = id else {
            anyhow::bail!("cheese extractor only supports cheese ids")
        };

        let value = api.get_cheese_season(*season_id).await?;
        Ok(parse_cheese_season_response(&value))
    }
}

/// 解析课程 season 响应 (结构同番剧, 但用 data 而非 result).
fn parse_cheese_season_response(v: &serde_json::Value) -> VInfo {
    let data = &v["data"];
    let episodes = data["episodes"].as_array();

    let pages: Vec<Page> = episodes
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let title = item["title"].as_str().unwrap_or("未命名");
                    let cid = item["cid"].as_u64().unwrap_or(0);
                    let aid = item["aid"].as_u64().unwrap_or(0);
                    let duration = item["duration"].as_u64().unwrap_or(0) as u32;
                    let width = item["dimension"]["width"].as_u64().unwrap_or(0) as u32;
                    let height = item["dimension"]["height"].as_u64().unwrap_or(0) as u32;
                    Page {
                        page_index: (aid as u32).max(1),
                        cid,
                        title: title.to_string(),
                        duration,
                        dimension: if width > 0 && height > 0 {
                            format!("{width}x{height}")
                        } else {
                            String::new()
                        },
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    VInfo {
        title: data["title"].as_str().unwrap_or("课程").to_string(),
        desc: data["subtitle"].as_str().unwrap_or("").to_string(),
        pic: data["cover"].as_str().unwrap_or("").to_string(),
        pubdate: data["pub_time"].as_str().map_or(0, parse_publish_time),
        owner_mid: 0,
        owner_name: String::new(),
        aid: 0,
        bvid: String::new(),
        cids: pages.iter().map(|page| page.cid).collect(),
        part_names: pages.iter().map(|page| page.title.clone()).collect(),
        pages,
        view_points: vec![],
        is_bangumi: false,
        is_cheese: true,
        is_stein_gate: false,
        tags: vec![],
    }
}

// ── FavListExtractor ──────────────────────────────────────────────

#[async_trait]
impl Extractor for FavListExtractor {
    fn name(&self) -> &'static str {
        "fav_list"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::Favourite { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let NormalizedId::Favourite { fid } = id else {
            anyhow::bail!("fav list extractor only supports favourite ids")
        };

        let value = api.get_fav_folder(*fid, 1).await?;
        let folder_title = value["data"]["title"]
            .as_str()
            .unwrap_or("收藏夹")
            .to_string();

        let medias = value["data"]["medias"].as_array();
        let pages: Vec<Page> = medias
            .map(|items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let title = item["title"].as_str().unwrap_or("未命名");
                        let _bvid = item["bvid"].as_str().unwrap_or(""); // collected in first_bvid
                        let cid = item["page"]["cid"].as_u64().unwrap_or(0);
                        let duration = item["duration"].as_u64().unwrap_or(0) as u32;
                        Page {
                            page_index: (i + 1) as u32,
                            cid,
                            title: title.to_string(),
                            duration,
                            dimension: String::new(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let first_bvid = medias
            .and_then(|items| items.first())
            .and_then(|item| item["bvid"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(VInfo {
            title: folder_title,
            desc: String::new(),
            pic: String::new(),
            pubdate: 0,
            owner_mid: 0,
            owner_name: String::new(),
            aid: 0,
            bvid: first_bvid,
            cids: pages.iter().map(|page| page.cid).collect(),
            part_names: pages.iter().map(|page| page.title.clone()).collect(),
            pages,
            view_points: vec![],
            is_bangumi: false,
            is_cheese: false,
            is_stein_gate: false,
            tags: vec![],
        })
    }
}

// ── MediaListExtractor ────────────────────────────────────────────

#[async_trait]
impl Extractor for MediaListExtractor {
    fn name(&self) -> &'static str {
        "media_list"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::MediaList { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let NormalizedId::MediaList { biz_id } = id else {
            anyhow::bail!("media list extractor only supports media list ids")
        };

        let value: serde_json::Value = api.get_media_list(*biz_id).await?;

        let ml_title = value["data"]["info"]["title"]
            .as_str()
            .unwrap_or("播单")
            .to_string();

        let media_list = value["data"]["media_list"].as_array();
        let pages: Vec<Page> = media_list
            .map(|items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let title = item["title"].as_str().unwrap_or("未命名");
                        let _bvid = item["bvid"].as_str().unwrap_or(""); // collected in first_bvid
                        let cid = item["page"]["cid"].as_u64().unwrap_or(0);
                        let duration = item["duration"].as_u64().unwrap_or(0) as u32;
                        Page {
                            page_index: (i + 1) as u32,
                            cid,
                            title: title.to_string(),
                            duration,
                            dimension: String::new(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let first_bvid = media_list
            .and_then(|items| items.first())
            .and_then(|item| item["bvid"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(VInfo {
            title: ml_title,
            desc: String::new(),
            pic: String::new(),
            pubdate: 0,
            owner_mid: 0,
            owner_name: String::new(),
            aid: 0,
            bvid: first_bvid,
            cids: pages.iter().map(|page| page.cid).collect(),
            part_names: pages.iter().map(|page| page.title.clone()).collect(),
            pages,
            view_points: vec![],
            is_bangumi: false,
            is_cheese: false,
            is_stein_gate: false,
            tags: vec![],
        })
    }
}

// ── SeriesExtractor ───────────────────────────────────────────────

#[async_trait]
impl Extractor for SeriesExtractor {
    fn name(&self) -> &'static str {
        "series"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::Series { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let NormalizedId::Series { sid } = id else {
            anyhow::bail!("series extractor only supports series ids")
        };

        let value = api.get_series(*sid).await?;
        let data = &value["data"];

        let series_title = data["meta"]["name"].as_str().unwrap_or("合集").to_string();

        let archives = data["archives"].as_array();
        let pages: Vec<Page> = archives
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        let title = item["title"].as_str().unwrap_or("未命名");
                        let cid = item["cid"].as_u64().unwrap_or(0);
                        let duration = item["duration"].as_u64().unwrap_or(0) as u32;
                        let width = item["dimension"]["width"].as_u64().unwrap_or(0) as u32;
                        let height = item["dimension"]["height"].as_u64().unwrap_or(0) as u32;
                        Page {
                            page_index: 0,
                            cid,
                            title: title.to_string(),
                            duration,
                            dimension: if width > 0 && height > 0 {
                                format!("{width}x{height}")
                            } else {
                                String::new()
                            },
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let first_bvid = archives
            .and_then(|items| items.first())
            .and_then(|item| item["bvid"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(VInfo {
            title: series_title,
            desc: String::new(),
            pic: data["meta"]["cover"].as_str().unwrap_or("").to_string(),
            pubdate: 0,
            owner_mid: data["meta"]["mid"].as_u64().unwrap_or(0),
            owner_name: data["meta"]["name"].as_str().unwrap_or("").to_string(),
            aid: 0,
            bvid: first_bvid,
            cids: pages.iter().map(|page| page.cid).collect(),
            part_names: pages.iter().map(|page| page.title.clone()).collect(),
            pages,
            view_points: vec![],
            is_bangumi: false,
            is_cheese: false,
            is_stein_gate: false,
            tags: vec![],
        })
    }
}

// ── CollectionExtractor ───────────────────────────────────────────

#[async_trait]
impl Extractor for CollectionExtractor {
    fn name(&self) -> &'static str {
        "collection"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::Collection { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let NormalizedId::Collection { mid, sid } = id else {
            anyhow::bail!("collection extractor only supports collection ids")
        };

        let value = api.get_collection(*mid, *sid, 1).await?;
        parse_collection_response(&value, *mid)
    }
}

// ── SpaceExtractor ────────────────────────────────────────────────

#[async_trait]
impl Extractor for SpaceExtractor {
    fn name(&self) -> &'static str {
        "space"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::Mid { .. } | NormalizedId::Space { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let mid = match id {
            NormalizedId::Space { mid } | NormalizedId::Mid { mid } => *mid,
            _ => anyhow::bail!("space extractor only supports space/mid ids"),
        };

        // 取前 30 个视频 (第一页)
        let value = api.get_space_archives(mid, 1).await?;
        let data = &value["data"];

        let owner_name = data["list"]["vlist"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item["author"].as_str())
            .unwrap_or("UP主")
            .to_string();

        let vlist = data["list"]["vlist"].as_array();
        let pages: Vec<Page> = vlist
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        let title = item["title"].as_str().unwrap_or("未命名");
                        let cid = item["cid"].as_u64().unwrap_or(0);
                        let duration = item["length"]
                            .as_str()
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0);
                        Page {
                            page_index: 0,
                            cid,
                            title: title.to_string(),
                            duration,
                            dimension: String::new(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let first_bvid = vlist
            .and_then(|items| items.first())
            .and_then(|item| item["bvid"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(VInfo {
            title: format!("{owner_name} 的空间投稿"),
            desc: String::new(),
            pic: String::new(),
            pubdate: 0,
            owner_mid: mid,
            owner_name,
            aid: 0,
            bvid: first_bvid,
            cids: pages.iter().map(|page| page.cid).collect(),
            part_names: pages.iter().map(|page| page.title.clone()).collect(),
            pages,
            view_points: vec![],
            is_bangumi: false,
            is_cheese: false,
            is_stein_gate: false,
            tags: vec![],
        })
    }
}

// ── IntlBangumiExtractor ──────────────────────────────────────────

#[async_trait]
impl Extractor for IntlBangumiExtractor {
    fn name(&self) -> &'static str {
        "intl_bangumi"
    }

    fn matches(&self, id: &NormalizedId) -> bool {
        matches!(id, NormalizedId::IntlBangumi { .. })
    }

    async fn extract(&self, id: &NormalizedId, api: &BilibiliApi) -> Result<VInfo> {
        let NormalizedId::IntlBangumi { season_id } = id else {
            anyhow::bail!("intl bangumi extractor only supports intl bangumi ids")
        };

        // v1.0: 国际版番剧复用国内番剧 API
        // 大多数国际版 season_id 与国内版兼容
        let value = api.get_bangumi_season(*season_id).await?;
        parse_bangumi_season_response(&value)
    }
}

/// 解析普通视频 view 响应.
pub fn parse_view_response(v: &serde_json::Value) -> Result<VInfo> {
    let envelope: ApiDataEnvelope<ViewPayload> = serde_json::from_value(v.clone())?;
    if envelope.code != 0 {
        anyhow::bail!("view api failed: {}", envelope.message);
    }

    let pages = envelope
        .data
        .pages
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    Ok(VInfo {
        title: envelope.data.title,
        desc: envelope.data.desc,
        pic: envelope.data.pic,
        pubdate: envelope.data.pubdate,
        owner_mid: envelope.data.owner.mid,
        owner_name: envelope.data.owner.name,
        aid: envelope.data.aid,
        bvid: envelope.data.bvid,
        cids: pages.iter().map(|page: &Page| page.cid).collect(),
        part_names: pages.iter().map(|page| page.title.clone()).collect(),
        pages,
        view_points: vec![],
        is_bangumi: false,
        is_cheese: false,
        is_stein_gate: envelope.data.rights.is_stein_gate != 0,
        tags: vec![],
    })
}

/// 解析 pagelist 响应.
pub fn parse_pagelist_response(v: &serde_json::Value) -> Result<Vec<Page>> {
    let envelope: ApiDataEnvelope<Vec<PagelistPayload>> = serde_json::from_value(v.clone())?;
    if envelope.code != 0 {
        anyhow::bail!("pagelist api failed: {}", envelope.message);
    }
    Ok(envelope.data.into_iter().map(Into::into).collect())
}

/// 解析番剧 season 响应.
pub fn parse_bangumi_season_response(v: &serde_json::Value) -> Result<VInfo> {
    let envelope: ApiResultEnvelope<BangumiPayload> = serde_json::from_value(v.clone())?;
    if envelope.code != 0 {
        anyhow::bail!("bangumi api failed: {}", envelope.message);
    }

    let pages = envelope
        .result
        .episodes
        .into_iter()
        .filter(|episode| episode.badge != "预告")
        .map(Into::into)
        .collect::<Vec<_>>();

    Ok(VInfo {
        title: envelope.result.title,
        desc: envelope.result.evaluate,
        pic: envelope.result.cover,
        pubdate: parse_publish_time(&envelope.result.publish.pub_time),
        owner_mid: 0,
        owner_name: String::new(),
        aid: 0,
        bvid: String::new(),
        cids: pages.iter().map(|page: &Page| page.cid).collect(),
        part_names: pages.iter().map(|page| page.title.clone()).collect(),
        pages,
        view_points: vec![],
        is_bangumi: true,
        is_cheese: false,
        is_stein_gate: false,
        tags: vec![],
    })
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ApiDataEnvelope<T> {
    code: i32,
    message: String,
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ApiResultEnvelope<T> {
    code: i32,
    message: String,
    result: T,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ViewPayload {
    title: String,
    desc: String,
    pic: String,
    pubdate: i64,
    aid: u64,
    bvid: String,
    owner: OwnerPayload,
    pages: Vec<ViewPagePayload>,
    rights: RightsPayload,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct OwnerPayload {
    mid: u64,
    name: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RightsPayload {
    is_stein_gate: u8,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ViewPagePayload {
    page: u32,
    cid: u64,
    part: String,
    duration: u32,
    dimension: DimensionPayload,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct PagelistPayload {
    page: u32,
    cid: u64,
    part: String,
    duration: u32,
    dimension: DimensionPayload,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct DimensionPayload {
    width: u32,
    height: u32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct BangumiPayload {
    cover: String,
    title: String,
    evaluate: String,
    publish: BangumiPublishPayload,
    episodes: Vec<BangumiEpisodePayload>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct BangumiPublishPayload {
    pub_time: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct BangumiEpisodePayload {
    aid: u64,
    cid: u64,
    id: u64,
    title: String,
    long_title: String,
    badge: String,
    pub_time: i64,
    dimension: DimensionPayload,
}

impl Default for ApiDataEnvelope<Vec<PagelistPayload>> {
    fn default() -> Self {
        Self {
            code: 0,
            message: String::new(),
            data: vec![],
        }
    }
}

impl Default for ApiDataEnvelope<ViewPayload> {
    fn default() -> Self {
        Self {
            code: 0,
            message: String::new(),
            data: ViewPayload::default(),
        }
    }
}

impl Default for ApiResultEnvelope<BangumiPayload> {
    fn default() -> Self {
        Self {
            code: 0,
            message: String::new(),
            result: BangumiPayload::default(),
        }
    }
}

impl From<ViewPagePayload> for Page {
    fn from(value: ViewPagePayload) -> Self {
        Self {
            page_index: value.page,
            cid: value.cid,
            title: value.part,
            duration: value.duration,
            dimension: format_dimension(&value.dimension),
        }
    }
}

impl From<PagelistPayload> for Page {
    fn from(value: PagelistPayload) -> Self {
        Self {
            page_index: value.page,
            cid: value.cid,
            title: value.part,
            duration: value.duration,
            dimension: format_dimension(&value.dimension),
        }
    }
}

impl From<BangumiEpisodePayload> for Page {
    fn from(value: BangumiEpisodePayload) -> Self {
        let title = format!("{} {}", value.title, value.long_title)
            .trim()
            .to_string();
        Self {
            page_index: 0,
            cid: value.cid,
            title,
            duration: 0,
            dimension: format_dimension(&value.dimension),
        }
    }
}

fn format_dimension(dimension: &DimensionPayload) -> String {
    if dimension.width == 0 || dimension.height == 0 {
        String::new()
    } else {
        format!("{}x{}", dimension.width, dimension.height)
    }
}

fn parse_publish_time(s: &str) -> i64 {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map_or(0, |dt| dt.and_utc().timestamp())
}

/// 解析合集 polymer API 响应.
pub fn parse_collection_response(v: &serde_json::Value, mid: u64) -> Result<VInfo> {
    let data = &v["data"];

    let collection_title = data["meta"]["name"].as_str().unwrap_or("合集").to_string();

    let archives = data["archives"].as_array();
    let pages: Vec<Page> = archives
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let title = item["title"].as_str().unwrap_or("未命名");
                    let cid = item["cid"].as_u64().unwrap_or(0);
                    let duration = item["duration"].as_u64().unwrap_or(0) as u32;
                    Page {
                        page_index: 0,
                        cid,
                        title: title.to_string(),
                        duration,
                        dimension: String::new(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let first_bvid = archives
        .and_then(|items| items.first())
        .and_then(|item| item["bvid"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(VInfo {
        title: collection_title,
        desc: String::new(),
        pic: data["meta"]["cover"].as_str().unwrap_or("").to_string(),
        pubdate: 0,
        owner_mid: mid,
        owner_name: String::new(),
        aid: 0,
        bvid: first_bvid,
        cids: pages.iter().map(|page| page.cid).collect(),
        part_names: pages.iter().map(|page| page.title.clone()).collect(),
        pages,
        view_points: vec![],
        is_bangumi: false,
        is_cheese: false,
        is_stein_gate: false,
        tags: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_defaults_finds_normal() {
        let registry = ExtractorRegistry::with_defaults();
        let id = NormalizedId::UgcVideo {
            id: "BV17x411w7KC".to_string(),
            page_index: 1,
        };
        assert_eq!(registry.find(&id).map(Extractor::name), Some("normal"));
    }

    #[test]
    fn test_registry_with_defaults_finds_bangumi() {
        let registry = ExtractorRegistry::with_defaults();
        let id = NormalizedId::Bangumi {
            season_id: Some(28234),
            ep_id: None,
            media_id: None,
        };
        assert_eq!(registry.find(&id).map(Extractor::name), Some("bangumi"));
    }

    #[test]
    fn test_registry_prefers_registered_order() {
        let registry = ExtractorRegistry::with_defaults();
        let id = NormalizedId::Space { mid: 42 };
        assert_eq!(registry.find(&id).map(Extractor::name), Some("space"));
    }

    #[test]
    fn test_parse_view_response() {
        let value = serde_json::json!({
            "code": 0,
            "message": "0",
            "data": {
                "title": "测试视频",
                "desc": "描述",
                "pic": "https://i0.hdslb.com/test.jpg",
                "pubdate": 1710000000,
                "aid": 170001,
                "bvid": "BV17x411w7KC",
                "owner": { "mid": 123, "name": "UP主" },
                "rights": { "is_stein_gate": 0 },
                "pages": [{
                    "page": 1,
                    "cid": 456,
                    "part": "P1",
                    "duration": 120,
                    "dimension": { "width": 1920, "height": 1080 }
                }]
            }
        });

        let info = parse_view_response(&value).unwrap();
        assert_eq!(info.title, "测试视频");
        assert_eq!(info.owner_name, "UP主");
        assert_eq!(info.bvid, "BV17x411w7KC");
        assert_eq!(info.cids, vec![456]);
    }

    #[test]
    fn test_parse_pagelist_response() {
        let value = serde_json::json!({
            "code": 0,
            "message": "0",
            "data": [{
                "page": 2,
                "cid": 789,
                "part": "P2",
                "duration": 99,
                "dimension": { "width": 1280, "height": 720 }
            }]
        });

        let pages = parse_pagelist_response(&value).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_index, 2);
        assert_eq!(pages[0].dimension, "1280x720");
    }

    #[test]
    fn test_registry_with_defaults_finds_collection() {
        let registry = ExtractorRegistry::with_defaults();
        let id = NormalizedId::Collection { mid: 123, sid: 456 };
        assert_eq!(registry.find(&id).map(Extractor::name), Some("collection"));
    }

    #[test]
    fn test_parse_bangumi_season_response() {
        let value = serde_json::json!({
            "code": 0,
            "message": "success",
            "result": {
                "cover": "https://i0.hdslb.com/bangumi.jpg",
                "title": "番剧标题",
                "evaluate": "番剧简介",
                "publish": { "pub_time": "2024-01-02 03:04:05" },
                "episodes": [{
                    "aid": 1,
                    "cid": 1001,
                    "id": 2001,
                    "title": "第1话",
                    "long_title": "启程",
                    "badge": "",
                    "pub_time": 1710000000,
                    "dimension": { "width": 1920, "height": 1080 }
                }]
            }
        });

        let info = parse_bangumi_season_response(&value).unwrap();
        assert!(info.is_bangumi);
        assert_eq!(info.title, "番剧标题");
        assert_eq!(info.pages[0].title, "第1话 启程");
    }

    #[test]
    fn test_collection_extractor_matches() {
        let extractor = CollectionExtractor;
        assert!(extractor.matches(&NormalizedId::Collection { mid: 1, sid: 2 }));
        assert!(!extractor.matches(&NormalizedId::UgcVideo {
            id: "BVxx".into(),
            page_index: 0
        }));
        assert!(!extractor.matches(&NormalizedId::Favourite { fid: 1 }));
    }

    #[test]
    fn test_parse_collection_response() {
        let value = serde_json::json!({
            "code": 0,
            "data": {
                "meta": {
                    "name": "我的合集",
                    "cover": "https://i0.hdslb.com/collection.jpg",
                    "mid": 12345
                },
                "archives": [
                    {
                        "bvid": "BV1xx411c7mD",
                        "aid": 170001,
                        "title": "合集视频1",
                        "cover": "https://i0.hdslb.com/v1.jpg",
                        "duration": 120,
                        "ctime": 1710000000,
                        "cid": 111
                    },
                    {
                        "bvid": "BV2xx411c7mE",
                        "aid": 170002,
                        "title": "合集视频2",
                        "cover": "https://i0.hdslb.com/v2.jpg",
                        "duration": 180,
                        "ctime": 1710000001,
                        "cid": 222
                    }
                ],
                "page": { "total": 2, "num": 1, "size": 30 }
            }
        });

        let info = parse_collection_response(&value, 12345).unwrap();
        assert_eq!(info.title, "我的合集");
        assert_eq!(info.bvid, "BV1xx411c7mD");
        assert_eq!(info.owner_mid, 12345);
        assert_eq!(info.cids, vec![111, 222]);
        assert_eq!(info.pages.len(), 2);
        assert_eq!(info.pages[0].title, "合集视频1");
        assert_eq!(info.pages[1].title, "合集视频2");
        assert_eq!(info.part_names, vec!["合集视频1", "合集视频2"]);
    }
}
