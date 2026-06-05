//! 重试策略 (基于 backoff 库).
//!
//! **算法**: 指数退避 + 抖动 (Exponential backoff with jitter).
//! - 初始: 500ms
//! - 倍数: 2x
//! - 最大: 30s
//! - 抖动: 0-100% 随机
//! - 最多: 3 次

use backoff::ExponentialBackoff;
use std::time::Duration;

/// 默认重试配置.
#[must_use]
pub fn default_backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        initial_interval: Duration::from_millis(500),
        max_interval: Duration::from_secs(30),
        max_elapsed_time: Some(Duration::from_secs(60)),
        multiplier: 2.0,
        randomization_factor: 0.5, // 0-100% 抖动
        ..Default::default()
    }
}

/// 激进重试 (用于 WBI 拉取等关键路径).
#[must_use]
pub fn aggressive_backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        initial_interval: Duration::from_millis(200),
        max_interval: Duration::from_secs(10),
        max_elapsed_time: Some(Duration::from_secs(30)),
        multiplier: 1.5,
        randomization_factor: 0.3,
        ..Default::default()
    }
}

/// 保守重试 (用于登录/大文件下载).
#[must_use]
pub fn conservative_backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        initial_interval: Duration::from_secs(2),
        max_interval: Duration::from_secs(60),
        max_elapsed_time: Some(Duration::from_secs(300)),
        multiplier: 2.0,
        randomization_factor: 0.5,
        ..Default::default()
    }
}

/// 异步执行 + 重试的便捷宏.
///
/// 用法:
/// ```ignore
/// let result: Result<MyData> = with_retry!(default_backoff(), {
///     reqwest::get("https://api.bilibili.com/...").await?.json().await
/// });
/// ```
#[macro_export]
macro_rules! with_retry {
    ($policy:expr, $body:block) => {{
        use backoff::backoff::Backoff;
        let mut bo = $policy;
        let mut attempt = 0u32;
        loop {
            match (|| async $body)().await {
                Ok(v) => break Ok::<_, anyhow::Error>(v),
                Err(e) => {
                    attempt += 1;
                    if attempt >= 3 {
                        break Err::<_, anyhow::Error>(anyhow::anyhow!("重试 {attempt} 次后仍失败: {e}"));
                    }
                    let delay = bo.next_backoff().unwrap_or(std::time::Duration::from_secs(5));
                    tracing::warn!("第 {attempt} 次失败, 等待 {delay:?} 后重试: {e}");
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_backoff() {
        let bo = default_backoff();
        assert_eq!(bo.initial_interval, Duration::from_millis(500));
    }

    #[test]
    fn test_aggressive_backoff() {
        let bo = aggressive_backoff();
        assert!(bo.max_elapsed_time.unwrap() < Duration::from_secs(60));
    }
}
