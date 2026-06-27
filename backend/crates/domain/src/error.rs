use thiserror::Error;

/// 采集源错误。domain 不依赖 reqwest，HTTP 错误由 adapter 转成 `Other`。
#[derive(Debug, Error)]
pub enum SourceError {
    #[error("rate limited")]
    RateLimited,
    #[error("cancelled")]
    Cancelled,
    #[error("source error: {0}")]
    Other(String),
}
