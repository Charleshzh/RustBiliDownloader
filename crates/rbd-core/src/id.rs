//! BilibiliId 类型化包装. 区分 AvId / BvId / CId / EpisodeId / SeasonId / MediaId / FId / SeriesId / MId.
//!
//! **算法来源**: 公开 B 站 BV<->AV 转换算法 (BBDown/Yutto/Pybilibili 一致实现).
//!   - XOR_CODE = 23442827791579
//!   - MASK_CODE = 2251799813685247
//!   - ALPHABET = "FcwAPNKTMug3GV5Lj7EJnHpWsx4tb8haYeviqBz6rkCy12mUSDQX9RdoZf"
//!   - ENCODE_MAP = (8, 7, 0, 5, 1, 3, 2, 4, 6)
//!
//! **URL 解析**: 完整支持 B 站 9 种 URL 形式 (BBDown/BilibiliUrlPatterns.cs 一致).
//!   - `https://www.bilibili.com/video/BVxxxxxxxxxx` (UGC)
//!   - `https://www.bilibili.com/video/BVxxxxxxxxxx?p=3` (分P)
//!   - `https://www.bilibili.com/bangumi/play/ss{ss_id}` (番剧)
//!   - `https://www.bilibili.com/bangumi/play/ep{ep_id}` (番剧单集)
//!   - `https://www.bilibili.com/cheese/play/ss{season_id}` (课程)
//!   - `https://www.bilibili.com/cheese/play/ep{ep_id}` (课程单集)
//!   - `https://space.bilibili.com/{mid}` (UP 主空间)
//!   - `https://space.bilibili.com/{mid}/favlist?fid={fid}` (收藏夹)
//!   - `https://www.bilibili.com/list/ml{biz_id}` (媒体列表)
//!   - `https://www.bilibili.com/series/bsc{sid}` (合集)
//!   - `BV17x411w7KC` / `av170001` (裸 ID)
//!   - `b23.tv/xxx` (短链 - 需 resolve)

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use url::Url;

/// 所有 Bilibili ID 的 trait.
pub trait BilibiliId: fmt::Display + fmt::Debug + Clone + Send + Sync {
    /// 原始字符串表示.
    fn raw(&self) -> &str;
}

/// 普通视频 AV 号 (纯数字).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AvId(pub u64);

/// BV 号 (B 站新格式, 12 字符 Base58).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BvId(pub String);

/// CID (视频物理分片 ID, 数字).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CId(pub u64);

/// 番剧单集 ID (ss + `ep_id`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EpisodeId(pub u64);

/// 番剧 season ID (ss).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SeasonId(pub u64);

/// 番剧 media ID (md).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MediaId(pub u64);

/// 收藏夹 ID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FId(pub u64);

/// 合集 ID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SeriesId(pub u64);

/// UP 主 mid.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MId(pub u64);

/// 短链 (b23.tv).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ShortUrl(pub String);

/// URL 解析后的归一化 ID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NormalizedId {
    /// 普通视频 (BV / AV)
    UgcVideo {
        /// BV 或 AV (BV 优先, 否则 "av" + 数字)
        id: String,
        /// 分 P 序号 (1-based, 0 = 整视频)
        page_index: u32,
    },
    /// 番剧 (ss / ep / md)
    Bangumi {
        /// season id
        season_id: Option<u64>,
        /// ep id
        ep_id: Option<u64>,
        /// media id
        media_id: Option<u64>,
    },
    /// 课程 (cheese)
    Cheese {
        /// season id
        season_id: u64,
        /// ep id
        ep_id: Option<u64>,
    },
    /// 收藏夹
    Favourite {
        /// fid
        fid: u64,
    },
    /// 合集
    Series {
        /// series id
        sid: u64,
    },
    /// 个人空间 (按 UP 主下载所有视频)
    Space {
        /// mid
        mid: u64,
    },
    /// 个人空间 mid (内部统一态, 与 Space 兼容)
    Mid {
        /// mid
        mid: u64,
    },
    /// 媒体列表 (歌单/播单)
    MediaList {
        /// biz id
        biz_id: u64,
    },
    /// 国际版番剧 season id
    IntlBangumi {
        /// season id
        season_id: u64,
    },
    /// 合集 (UP 主创建的合集, `season_id`)
    Collection {
        /// mid
        mid: u64,
        /// `season_id` (collection sid)
        sid: u64,
    },
    /// 短链 (需 b23.tv → 完整 URL 重定向后再次 parse)
    ShortLink {
        /// 短码 (b23.tv/ 后部分)
        code: String,
    },
}

impl fmt::Display for NormalizedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NormalizedId::UgcVideo { id, page_index } => {
                if *page_index > 0 {
                    write!(f, "UGC 视频: {id} (P{page_index})")
                } else {
                    write!(f, "UGC 视频: {id}")
                }
            }
            NormalizedId::Bangumi {
                season_id: Some(ss),
                ep_id: Some(ep),
                ..
            } => write!(f, "番剧: ss{ss} ep{ep}"),
            NormalizedId::Bangumi {
                season_id: Some(ss),
                ..
            } => write!(f, "番剧: ss{ss}"),
            NormalizedId::Bangumi {
                ep_id: Some(ep), ..
            } => write!(f, "番剧单集: ep{ep}"),
            NormalizedId::Bangumi {
                media_id: Some(md), ..
            } => write!(f, "番剧: md{md}"),
            NormalizedId::Bangumi { .. } => write!(f, "番剧"),
            NormalizedId::Cheese {
                season_id,
                ep_id: Some(ep),
            } => write!(f, "课程: ss{season_id} ep{ep}"),
            NormalizedId::Cheese { season_id, .. } => write!(f, "课程: ss{season_id}"),
            NormalizedId::Favourite { fid } => write!(f, "收藏夹: fid{fid}"),
            NormalizedId::Series { sid } => write!(f, "合集: bsc{sid}"),
            NormalizedId::Space { mid } => write!(f, "UP 主空间: mid{mid}"),
            NormalizedId::Mid { mid } => write!(f, "UP 主: mid{mid}"),
            NormalizedId::MediaList { biz_id } => write!(f, "媒体列表: ml{biz_id}"),
            NormalizedId::IntlBangumi { season_id } => write!(f, "国际版番剧: ss{season_id}"),
            NormalizedId::Collection { mid, sid } => write!(f, "合集: mid{mid} sid{sid}"),
            NormalizedId::ShortLink { code } => write!(f, "短链: {code}"),
        }
    }
}

impl fmt::Display for AvId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "av{}", self.0)
    }
}

impl fmt::Display for BvId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for CId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cid={}", self.0)
    }
}

impl FromStr for BvId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        crate::bv::validate_bv(s)?;
        Ok(Self(s.to_string()))
    }
}

impl FromStr for AvId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        let n: u64 = s.trim_start_matches("av").parse()?;
        Ok(Self(n))
    }
}

impl BilibiliId for BvId {
    fn raw(&self) -> &str {
        &self.0
    }
}

impl BilibiliId for AvId {
    fn raw(&self) -> &'static str {
        // 不可返回 &str, 退化
        ""
    }
}

// === URL 解析 ===

// 静态正则: 一次性编译, 全部用 once_cell 缓存
static RE_VIDEO_URL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://www.bilibili.com/video/BV1xx411c7mD 或 av170001
    Regex::new(r"bilibili\.com/video/(?P<id>(BV[1-9A-HJ-NP-Za-km-z]{10}|av\d+))").unwrap()
});
static RE_BANGUMI_SS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://www.bilibili.com/bangumi/play/ss123
    Regex::new(r"bilibili\.com/bangumi/play/ss(?P<ss>\d+)").unwrap()
});
static RE_BANGUMI_EP: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://www.bilibili.com/bangumi/play/ep123
    Regex::new(r"bilibili\.com/bangumi/play/ep(?P<ep>\d+)").unwrap()
});
static RE_BANGUMI_MD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://www.bilibili.com/bangumi/media/md123
    Regex::new(r"bilibili\.com/bangumi/media/md(?P<md>\d+)").unwrap()
});
static RE_CHEESE_SS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://www.bilibili.com/cheese/play/ss123
    Regex::new(r"bilibili\.com/cheese/play/ss(?P<ss>\d+)").unwrap()
});
static RE_CHEESE_EP: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://www.bilibili.com/cheese/play/ep123
    Regex::new(r"bilibili\.com/cheese/play/ep(?P<ep>\d+)").unwrap()
});
static RE_FAV: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://space.bilibili.com/123/favlist?fid=456
    Regex::new(r"space\.bilibili\.com/(?P<mid>\d+)/favlist\?.*?fid=(?P<fid>\d+)").unwrap()
});
static RE_COLLECTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://space.bilibili.com/123/collectiondetail?sid=456
    Regex::new(r"space\.bilibili\.com/(?P<mid>\d+)/collectiondetail\?.*?sid=(?P<sid>\d+)").unwrap()
});
static RE_SPACE_VIDEO: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://space.bilibili.com/123/video
    Regex::new(r"space\.bilibili\.com/(?P<mid>\d+)(/|$|\?|#)").unwrap()
});
static RE_MEDIA_LIST: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://www.bilibili.com/list/ml12345
    Regex::new(r"bilibili\.com/list/ml(?P<id>\d+)").unwrap()
});
static RE_SERIES: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://www.bilibili.com/series/bsc12345
    Regex::new(r"bilibili\.com/series/(?P<sid>\d+|bsc\d+)").unwrap()
});
static RE_SHORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // https://b23.tv/xxx 或 https://bili2233.cn/xxx
    Regex::new(r"(b23\.tv|bili2233\.cn)/(?P<code>[A-Za-z0-9]+)").unwrap()
});
static RE_RAW_BV: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"^BV[1-9A-HJ-NP-Za-km-z]{10}$").unwrap());
static RE_RAW_AV: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"^av\d+$").unwrap());

/// 解析 B 站 URL → `NormalizedId`. 解析失败返回 `anyhow::Error`.
pub fn parse_url(input: &str) -> anyhow::Result<NormalizedId> {
    let s = input.trim();
    if s.is_empty() {
        anyhow::bail!("URL 不能为空");
    }

    // 短链: 单独识别, 需 HTTP resolve 后再 parse
    if let Some(c) = RE_SHORT.captures(s) {
        return Ok(NormalizedId::ShortLink {
            code: c.name("code").unwrap().as_str().to_string(),
        });
    }

    // 裸 BV 号
    if RE_RAW_BV.is_match(s) {
        return Ok(NormalizedId::UgcVideo {
            id: s.to_string(),
            page_index: 0,
        });
    }
    // 裸 AV 号
    if RE_RAW_AV.is_match(s) {
        return Ok(NormalizedId::UgcVideo {
            id: s.to_string(),
            page_index: 0,
        });
    }

    // 番剧 media
    if let Some(c) = RE_BANGUMI_MD.captures(s) {
        return Ok(NormalizedId::Bangumi {
            season_id: None,
            ep_id: None,
            media_id: Some(c.name("md").unwrap().as_str().parse()?),
        });
    }
    // 番剧 ss
    if let Some(c) = RE_BANGUMI_SS.captures(s) {
        return Ok(NormalizedId::Bangumi {
            season_id: Some(c.name("ss").unwrap().as_str().parse()?),
            ep_id: None,
            media_id: None,
        });
    }
    // 番剧 ep
    if let Some(c) = RE_BANGUMI_EP.captures(s) {
        return Ok(NormalizedId::Bangumi {
            season_id: None,
            ep_id: Some(c.name("ep").unwrap().as_str().parse()?),
            media_id: None,
        });
    }

    // 课程 ss
    if let Some(c) = RE_CHEESE_SS.captures(s) {
        return Ok(NormalizedId::Cheese {
            season_id: c.name("ss").unwrap().as_str().parse()?,
            ep_id: None,
        });
    }
    // 课程 ep
    if let Some(c) = RE_CHEESE_EP.captures(s) {
        return Ok(NormalizedId::Cheese {
            season_id: 0,
            ep_id: Some(c.name("ep").unwrap().as_str().parse()?),
        });
    }

    // 合集 (在收藏夹前, 因为更具体)
    if let Some(c) = RE_COLLECTION.captures(s) {
        return Ok(NormalizedId::Collection {
            mid: c.name("mid").unwrap().as_str().parse()?,
            sid: c.name("sid").unwrap().as_str().parse()?,
        });
    }

    // 收藏夹 (优先于 space 匹配, 因为更具体)
    if let Some(c) = RE_FAV.captures(s) {
        return Ok(NormalizedId::Favourite {
            fid: c.name("fid").unwrap().as_str().parse()?,
        });
    }

    // 媒体列表
    if let Some(c) = RE_MEDIA_LIST.captures(s) {
        return Ok(NormalizedId::MediaList {
            biz_id: c.name("id").unwrap().as_str().parse()?,
        });
    }

    // 合集
    if let Some(c) = RE_SERIES.captures(s) {
        let raw = c.name("sid").unwrap().as_str();
        let sid: u64 = raw.trim_start_matches("bsc").parse()?;
        return Ok(NormalizedId::Series { sid });
    }

    // UGC 视频
    if let Some(c) = RE_VIDEO_URL.captures(s) {
        let id = c.name("id").unwrap().as_str().to_string();
        let page_index = extract_page_index(s);
        return Ok(NormalizedId::UgcVideo { id, page_index });
    }

    // UP 主空间 (放最后, 避免被 favlist 误匹配)
    if let Some(c) = RE_SPACE_VIDEO.captures(s) {
        return Ok(NormalizedId::Space {
            mid: c.name("mid").unwrap().as_str().parse()?,
        });
    }

    // 兜底: 尝试用 url::Url 解析, 失败才报错
    if let Ok(u) = Url::parse(s) {
        anyhow::bail!(
            "无法识别 B 站 URL: {s} (host={})",
            u.host_str().unwrap_or("")
        );
    }
    anyhow::bail!("无法识别 B 站 URL: {s}");
}

/// 提取 URL 中的 `?p=N` 或 `&p=N` 参数.
fn extract_page_index(s: &str) -> u32 {
    // 找 "p=" 后跟数字
    if let Some(pos) = s.find("?p=").or_else(|| s.find("&p=")) {
        let after = &s[pos + 3..];
        let num: String = after.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = num.parse::<u32>() {
            return n;
        }
    }
    // 部分番剧页用 ?p=ep_id.episode 形式 (e.g. ?p=ep123.1)
    if let Some(pos) = s.find("?p=ep") {
        let after = &s[pos + 5..];
        // 跳过数字找 "."
        let num: String = after.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = num.parse::<u32>() {
            return n;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ugc_bv() {
        match parse_url("https://www.bilibili.com/video/BV17x411w7KC").unwrap() {
            NormalizedId::UgcVideo { id, page_index } => {
                assert_eq!(id, "BV17x411w7KC");
                assert_eq!(page_index, 0);
            }
            _ => panic!("expected UgcVideo"),
        }
    }

    #[test]
    fn test_ugc_av() {
        match parse_url("https://www.bilibili.com/video/av170001").unwrap() {
            NormalizedId::UgcVideo { id, page_index } => {
                assert_eq!(id, "av170001");
                assert_eq!(page_index, 0);
            }
            _ => panic!("expected UgcVideo"),
        }
    }

    #[test]
    fn test_ugc_with_page() {
        match parse_url("https://www.bilibili.com/video/BV17x411w7KC?p=3").unwrap() {
            NormalizedId::UgcVideo { id, page_index } => {
                assert_eq!(id, "BV17x411w7KC");
                assert_eq!(page_index, 3);
            }
            _ => panic!("expected UgcVideo"),
        }
    }

    #[test]
    fn test_raw_bv() {
        match parse_url("BV17x411w7KC").unwrap() {
            NormalizedId::UgcVideo { id, page_index } => {
                assert_eq!(id, "BV17x411w7KC");
                assert_eq!(page_index, 0);
            }
            _ => panic!("expected UgcVideo"),
        }
    }

    #[test]
    fn test_bangumi_ss() {
        match parse_url("https://www.bilibili.com/bangumi/play/ss12345").unwrap() {
            NormalizedId::Bangumi {
                season_id,
                ep_id,
                media_id,
            } => {
                assert_eq!(season_id, Some(12345));
                assert_eq!(ep_id, None);
                assert_eq!(media_id, None);
            }
            _ => panic!("expected Bangumi"),
        }
    }

    #[test]
    fn test_bangumi_ep() {
        match parse_url("https://www.bilibili.com/bangumi/play/ep67890").unwrap() {
            NormalizedId::Bangumi {
                season_id,
                ep_id,
                media_id,
            } => {
                assert_eq!(season_id, None);
                assert_eq!(ep_id, Some(67890));
                assert_eq!(media_id, None);
            }
            _ => panic!("expected Bangumi"),
        }
    }

    #[test]
    fn test_bangumi_md() {
        match parse_url("https://www.bilibili.com/bangumi/media/md28234").unwrap() {
            NormalizedId::Bangumi {
                season_id,
                ep_id,
                media_id,
            } => {
                assert_eq!(media_id, Some(28234));
                assert_eq!(season_id, None);
                assert_eq!(ep_id, None);
            }
            _ => panic!("expected Bangumi"),
        }
    }

    #[test]
    fn test_cheese_ss() {
        match parse_url("https://www.bilibili.com/cheese/play/ss888").unwrap() {
            NormalizedId::Cheese { season_id, ep_id } => {
                assert_eq!(season_id, 888);
                assert_eq!(ep_id, None);
            }
            _ => panic!("expected Cheese"),
        }
    }

    #[test]
    fn test_fav() {
        match parse_url("https://space.bilibili.com/12345/favlist?fid=67890").unwrap() {
            NormalizedId::Favourite { fid } => {
                assert_eq!(fid, 67890);
            }
            _ => panic!("expected Favourite"),
        }
    }

    #[test]
    fn test_space() {
        match parse_url("https://space.bilibili.com/12345").unwrap() {
            NormalizedId::Space { mid } => {
                assert_eq!(mid, 12345);
            }
            _ => panic!("expected Space"),
        }
    }

    #[test]
    fn test_media_list() {
        match parse_url("https://www.bilibili.com/list/ml12345").unwrap() {
            NormalizedId::MediaList { biz_id } => {
                assert_eq!(biz_id, 12345);
            }
            _ => panic!("expected MediaList"),
        }
    }

    #[test]
    fn test_series() {
        match parse_url("https://www.bilibili.com/series/bsc12345").unwrap() {
            NormalizedId::Series { sid } => {
                assert_eq!(sid, 12345);
            }
            _ => panic!("expected Series"),
        }
    }

    #[test]
    fn test_short_link() {
        match parse_url("https://b23.tv/abc123").unwrap() {
            NormalizedId::ShortLink { code } => {
                assert_eq!(code, "abc123");
            }
            _ => panic!("expected ShortLink"),
        }
    }

    #[test]
    fn test_collection() {
        match parse_url("https://space.bilibili.com/12345/collectiondetail?sid=67890").unwrap() {
            NormalizedId::Collection { mid, sid } => {
                assert_eq!(mid, 12345);
                assert_eq!(sid, 67890);
            }
            _ => panic!("expected Collection"),
        }
    }

    #[test]
    fn test_collection_with_extra_params() {
        match parse_url("https://space.bilibili.com/777/collectiondetail?sid=888&tab=1").unwrap() {
            NormalizedId::Collection { mid, sid } => {
                assert_eq!(mid, 777);
                assert_eq!(sid, 888);
            }
            _ => panic!("expected Collection"),
        }
    }

    #[test]
    fn test_invalid() {
        assert!(parse_url("https://example.com/").is_err());
        assert!(parse_url("not a url").is_err());
        assert!(parse_url("").is_err());
    }
}
