//! 限流抽象。生产用 governor 漏桶，测试用 NoLimit 跳过等待。

use async_trait::async_trait;
use governor::{
    clock::DefaultClock, middleware::NoOpMiddleware, state::InMemoryState, state::NotKeyed, Quota,
    RateLimiter,
};
use std::num::NonZeroU32;

#[async_trait]
pub trait RateLimit: Send + Sync {
    async fn acquire(&self);
}

type Governor = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// governor 漏桶限流器。克隆共享状态，故直接 clone 即可多处共用。
pub struct GovernorLimiter(Governor);

impl GovernorLimiter {
    pub fn per_second(nps: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(nps).expect("nps > 0"));
        Self(RateLimiter::direct(quota))
    }
}

#[async_trait]
impl RateLimit for GovernorLimiter {
    async fn acquire(&self) {
        self.0.until_ready().await;
    }
}

/// 不限流，测试用。
pub struct NoLimit;

#[async_trait]
impl RateLimit for NoLimit {
    async fn acquire(&self) {}
}
