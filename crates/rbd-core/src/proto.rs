//! gRPC 协议/枚举定义 — 备选 APP 端, M4 启用. 现仅占位, 保留 tonic 依赖.

/// APP 端 gRPC 模式 (5-mode playurl 选择之一).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiMode {
    /// WEB API.
    Web,
    /// APP API.
    App,
    /// TV API.
    Tv,
    /// 国际版 API.
    Intl,
    /// 自动选择.
    Auto,
}

impl ApiMode {
    /// 从字符串解析模式.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "web" => Self::Web,
            "app" => Self::App,
            "tv" => Self::Tv,
            "intl" => Self::Intl,
            _ => Self::Auto,
        }
    }

    /// 返回模式字符串.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::App => "app",
            Self::Tv => "tv",
            Self::Intl => "intl",
            Self::Auto => "auto",
        }
    }
}

impl Default for ApiMode {
    fn default() -> Self {
        Self::Web
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_mode_roundtrip() {
        for mode in [
            ApiMode::Web,
            ApiMode::App,
            ApiMode::Tv,
            ApiMode::Intl,
            ApiMode::Auto,
        ] {
            assert_eq!(ApiMode::parse(mode.as_str()), mode);
        }
    }

    #[test]
    fn test_api_mode_case_insensitive() {
        assert_eq!(ApiMode::parse("WEB"), ApiMode::Web);
        assert_eq!(ApiMode::parse("App"), ApiMode::App);
    }

    #[test]
    fn test_api_mode_unknown_defaults_auto() {
        assert_eq!(ApiMode::parse("xyz"), ApiMode::Auto);
        assert_eq!(ApiMode::parse(""), ApiMode::Auto);
    }
}
