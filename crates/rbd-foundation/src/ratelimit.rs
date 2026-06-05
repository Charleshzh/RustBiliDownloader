//! 限流器 (基于 governor).
//!
//! - 默认 50 req/s (未登录) / 200 req/s (登录)
//! - 限流是 async, 不会阻塞当前线程
//!
//! **算法来源**: governor crate (Jepsen 风格令牌桶)

use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::time::Duration;

/// governor 的 RateLimiter 类型别名.
pub type RateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// 创建默认 RateLimiter.
///
/// `rps`: 每秒请求数
#[must_use]
pub fn new_limiter(rps: u32) -> RateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(rps.max(1)).unwrap());
    GovernorRateLimiter::direct(quota)
}

/// 异步等待令牌 (1 个).
pub async fn tick(limiter: &RateLimiter) {
    limiter.until_ready().await;
}

/// 异步等待 N 个令牌.
pub async fn tick_n(limiter: &RateLimiter, n: u32) {
    for _ in 0..n {
        limiter.until_ready().await;
    }
}

/// 推荐 rps: 未登录 50, 登录 200.
#[must_use]
pub fn recommended_rps(logged_in: bool) -> u32 {
    if logged_in { 200 } else { 50 }
}

/// 两个请求之间的最小间隔 (`Duration`).
#[must_use]
pub fn min_interval(logged_in: bool) -> Duration {
    Duration::from_millis(1000 / u64::from(recommended_rps(logged_in)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_limiter_basic() {
        let lim = new_limiter(10);
        for _ in 0..5 {
            tick(&lim).await;
        }
    }

    #[test]
    fn test_recommended_rps() {
        assert_eq!(recommended_rps(false), 50);
        assert_eq!(recommended_rps(true), 200);
    }
}
