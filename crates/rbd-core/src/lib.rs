//! # rbd-core
//!
//! RBD 核心协议层: BilibiliId / WBI 签名 / BV<->AV / 8 个 Extractor / 5-mode playurl 调度.
//!
//! 这是协议核心, **无 I/O 副作用**, 可独立单元测试.

#![warn(missing_docs)]

/// 检查 URL 是否可安全下载 (防 SSRF).
///
/// 拦截以下地址:
/// - 非 HTTP/HTTPS 协议
/// - localhost / 127.0.0.0/8
/// - 私有网络: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
/// - 链路本地: 169.254.0.0/16
/// - 其他保留地址: 0.0.0.0/8
/// - IPv6 loopback `[::1]`
/// - .local / .internal 域名
pub fn is_safe_download_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if !matches!(parsed.scheme(), "https" | "http") {
        return false;
    }
    let Some(host) = parsed.host_str() else {
        return false;
    };

    // 阻断 localhost / loopback
    if host == "localhost"
        || host == "127.0.0.1"
        || host == "0.0.0.0"
        || host == "[::1]"
        || host.starts_with("127.")
        || host.starts_with("0.")
    {
        return false;
    }

    // 阻断私有网络 192.168.0.0/16
    if host.starts_with("192.168.") {
        return false;
    }

    // 阻断私有网络 10.0.0.0/8
    if host.starts_with("10.") {
        return false;
    }

    // 阻断私有网络 172.16.0.0/12
    if host.starts_with("172.") {
        if let Some(second) = host.split('.').nth(1).and_then(|s| s.parse::<u32>().ok()) {
            if (16..=31).contains(&second) {
                return false;
            }
        }
    }

    // 阻断链路本地 169.254.0.0/16
    if host.starts_with("169.254.") {
        return false;
    }

    // 阻断 mDNS / 内网域名
    if host.ends_with(".local") || host.ends_with(".internal") {
        return false;
    }

    true
}

/// Bilibili 9 种 ID 类型 (AvId / BvId / CId / SeasonId / ...).
pub mod id;
/// BV ↔ AV 转换算法.
pub mod bv;
/// WBI 签名算法.
pub mod wbi;
/// Extractor trait + 8 个 Fetcher.
pub mod extractor;
/// playurl 解析 (DASH 视频/音频流).
pub mod playurl;
/// 核心数据模型 (VInfo / Page / Track).
pub mod model;
/// B 站 WEB API 客户端 (view / pagelist / playurl / subtitle).
pub mod api;
/// gRPC 协议/枚举定义 (备选 APP 端, M4 启用).
pub mod proto;

pub use id::BilibiliId;
pub use api::BilibiliApi;
pub use model::{VInfo, Page, ViewPoint, Track, AudioTrack, VideoTrack, SubtitleTrack};
pub use playurl::{parse_playurl, PlayUrlResponse};
pub use proto::ApiMode;
pub use extractor::{Extractor, ExtractorRegistry};
pub use wbi::WbiKey;
