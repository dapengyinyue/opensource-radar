//! 通知器抽象。多通道推送（Server酱/飞书/钉钉…）的统一接口。

use async_trait::async_trait;
use thiserror::Error;

/// 通知发送错误。domain 不依赖 reqwest，HTTP 错误由 adapter 转成 `Http(String)`。
#[derive(Debug, Error)]
pub enum NotifyError {
    #[error("http error: {0}")]
    Http(String),
    #[error("api error: code={code} message={message}")]
    Api { code: i64, message: String },
    #[error("not configured")]
    NotConfigured,
}

/// 通知器：发送一条标题 + markdown 正文的消息。
/// 新通道只需实现本 trait 并在装配处注入。
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, title: &str, desp: &str) -> Result<(), NotifyError>;
}
