//! # rbd-core
//!
//! RBD 核心协议层: `BilibiliId` / WBI 签名 / BV<->AV / 8 个 Extractor / 5-mode playurl 调度.
//!
//! 这是协议核心, 包含 SSRF 防护和 DNS Rebinding 防御.

#![warn(missing_docs)]

use std::net::ToSocketAddrs;

/// 检查 URL 是否可安全下载 (防 SSRF).
///
/// 拦截以下地址:
/// - 非 HTTP/HTTPS 协议
/// - localhost / 127.0.0.0/8
/// - 私有网络: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
/// - 链路本地: 169.254.0.0/16
/// - 其他保留地址: 0.0.0.0/8
/// - IPv6 loopback `::1` / `[::1]`
/// - IPv6-mapped IPv4 loopback: `::ffff:127.0.0.1`, `::ffff:0.0.0.0`
/// - IPv6 link-local `fe80::/10`
/// - IPv6 unique local `fc00::/7`
/// - .local / .internal 域名
#[must_use]
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
        || host == "::1"
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

    // 阻断 IPv6-mapped IPv4 loopback / unspecified
    if host == "::ffff:127.0.0.1" || host == "::ffff:0.0.0.0" {
        return false;
    }

    // 阻断 IPv6 地址中的 link-local (fe80::/10) 和 unique local (fc00::/7)
    // 仅对包含 : 且不含 . 的主机名检查 (粗略区分 IPv6 地址和域名)
    if host.contains(':') && !host.contains('.') {
        let lower = host.to_lowercase();
        // fe80::/10: fe80 到 febf
        if lower.len() >= 4 {
            let prefix = &lower[..4];
            if prefix.starts_with("fe8")
                || prefix.starts_with("fe9")
                || prefix.starts_with("fea")
                || prefix.starts_with("feb")
            {
                return false;
            }
        }
        // fc00::/7: fc00 到 fdff
        if lower.len() >= 2 {
            let prefix = &lower[..2];
            if prefix == "fc" || prefix == "fd" {
                return false;
            }
        }
    }

    // 阻断 mDNS / 内网域名
    if std::path::Path::new(host)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("local"))
        || std::path::Path::new(host)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("internal"))
    {
        return false;
    }

    true
}

/// 解析主机名并验证 IP 地址安全 (防 DNS Rebinding).
///
/// 在下载前解析主机名, 检查解析后的所有 IP 地址是否属于黑名单
/// (回环 / 私有 / 链路本地 / 未指定 / IPv6 ULA).
///
/// DNS Rebinding 攻击: 攻击者控制域名的 DNS, 第一次解析返回合法 IP,
/// 第二次解析返回内网 IP. `is_safe_download_url()` 仅做字符串匹配,
/// 本函数在下载前做真实的 DNS 解析并验证, 构成双重防线.
pub fn resolve_and_validate_ip(url: &str) -> anyhow::Result<()> {
    let parsed = url::Url::parse(url).map_err(|e| anyhow::anyhow!("URL 解析失败: {e}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("URL 缺少主机名"))?;

    // 使用标准库 DNS 解析 (阻塞式, 在 spawn_blocking 或小流量场景下可接受)
    let addrs: Vec<std::net::SocketAddr> = (host, 0u16)
        .to_socket_addrs()
        .map_err(|e| anyhow::anyhow!("DNS 解析失败 ({host}): {e}"))?
        .collect();

    for addr in &addrs {
        let ip = addr.ip();
        if is_ip_blacklisted(&ip) {
            return Err(anyhow::anyhow!(
                "DNS 解析到禁止的 IP 地址 {ip} (主机: {host}), 拒绝连接"
            ));
        }
    }

    Ok(())
}

/// 检查 IP 地址是否在黑名单中 (回环 / 私有 / 链路本地 / 未指定 / ULA).
fn is_ip_blacklisted(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
        }
        std::net::IpAddr::V6(v6) => {
            // IPv6 loopback ::1
            if v6.is_loopback() {
                return true;
            }
            // IPv6-mapped IPv4: check the mapped IPv4 whether private
            if let Some(mapped_v4) = v6.to_ipv4() {
                return mapped_v4.is_loopback()
                    || mapped_v4.is_private()
                    || mapped_v4.is_link_local()
                    || mapped_v4.is_unspecified();
            }
            let segments = v6.segments();
            // IPv6 link-local fe80::/10: first 10 bits == 1111_1110_10
            if (segments[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            // IPv6 unique local fc00::/7: first 7 bits == 1111_110
            if (segments[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            false
        }
    }
}

/// B 站 WEB API 客户端 (view / pagelist / playurl / subtitle).
pub mod api;
/// BV ↔ AV 转换算法.
pub mod bv;
/// Extractor trait + 8 个 Fetcher.
pub mod extractor;
/// Bilibili 9 种 ID 类型 (AvId / BvId / CId / SeasonId / ...).
pub mod id;
/// 核心数据模型 (VInfo / Page / Track).
pub mod model;
/// playurl 解析 (DASH 视频/音频流).
pub mod playurl;
/// gRPC 协议/枚举定义 (备选 APP 端, M4 启用).
pub mod proto;
/// WBI 签名算法.
pub mod wbi;

pub use api::BilibiliApi;
pub use extractor::{Extractor, ExtractorRegistry};
pub use id::BilibiliId;
pub use model::{AudioTrack, Page, SubtitleTrack, Track, VInfo, VideoTrack, ViewPoint};
pub use playurl::{parse_playurl, PlayUrlResponse};
pub use proto::ApiMode;
pub use wbi::WbiKey;
