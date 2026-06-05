//! 统一错误类型. 库内用 [`RbdError`], 二进制边界 (CLI) 用 [`anyhow::Error`] 包装.

use thiserror::Error;

/// RBD 库的统一错误类型.
///
/// 部分变体将在 v2.0 全面接入各个 crate, 当前 anyhow 兜底.
#[derive(Debug, Error)]
pub enum RbdError {
    /// 网络错误 (含超时, TLS, 连接重置)
    #[error("网络错误: {0}")]
    Network(String),

    /// HTTP 状态码非 2xx
    #[error("HTTP {status}: {message}")]
    Http {
        /// HTTP 状态码
        status: u16,
        /// 响应体 (前 200 字节)
        message: String,
    },

    /// 429 限流, 含 `Retry-After` 提示
    #[error("限流 (429), 建议等待 {retry_after_secs}s")]
    RateLimited {
        /// 推荐等待秒数
        retry_after_secs: u64,
    },

    /// JSON 解析失败
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),

    /// URL 解析失败
    #[error("URL 解析失败: {0}")]
    Url(#[from] url::ParseError),

    /// I/O 错误
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    /// 配置错误
    #[allow(dead_code)]
    #[error("配置错误: {0}")]
    Config(String),

    /// 鉴权失败 (cookie 失效 / token 过期 / 需扫码)
    #[allow(dead_code)]
    #[error("鉴权失败: {0}")]
    Auth(String),

    /// WBI 签名失败
    #[allow(dead_code)]
    #[error("WBI 签名失败: {0}")]
    Wbi(String),

    /// 视频不存在 / 被删除 / 不可见
    #[allow(dead_code)]
    #[error("视频不可访问: {0}")]
    NotFound(String),

    /// 不支持的视频类型 (URL 解析成功但无 extractor 匹配)
    #[allow(dead_code)]
    #[error("不支持的 URL 类型: {0}")]
    UnsupportedUrl(String),

    /// 杜比视界需要 ffmpeg 5.0+
    #[allow(dead_code)]
    #[error("杜比视界需要 ffmpeg 5.0+ (当前: {current})")]
    NeedNewFfmpeg {
        /// 当前 ffmpeg 版本字符串
        current: String,
    },

    /// 用户取消 (Ctrl+C)
    #[allow(dead_code)]
    #[error("用户取消")]
    Cancelled,

    /// 下载片段校验失败 (Content-Length 不匹配)
    #[allow(dead_code)]
    #[error("下载片段校验失败: 期望 {expected} 字节, 实际 {actual} 字节")]
    ChunkSizeMismatch {
        /// 期望字节数
        expected: u64,
        /// 实际字节数
        actual: u64,
    },

    /// ffmpeg 调用失败
    #[error("ffmpeg 调用失败: {0}")]
    Ffmpeg(String),

    /// 密钥环访问失败 (Linux 无 dbus / Windows 凭据管理器被禁用)
    #[allow(dead_code)]
    #[error("密钥环访问失败: {0}")]
    Keyring(String),

    /// 其他错误
    #[allow(dead_code)]
    #[error("{0}")]
    Other(String),
}

/// RBD Result 别名.
pub type Result<T> = std::result::Result<T, RbdError>;
