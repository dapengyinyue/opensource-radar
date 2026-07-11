//! 日报调度：每天固定时刻生成并发送日报。

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use domain::notifier::Notifier;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::digest::{generate_digest, next_digest_delay};

const DAY: Duration = Duration::from_secs(24 * 3600);

pub struct DigestScheduler {
    pool: PgPool,
    notifier: Arc<dyn Notifier>,
    hour: u32,
    top_n: i64,
    cancel: CancellationToken,
}

impl DigestScheduler {
    pub fn new(
        pool: PgPool,
        notifier: Arc<dyn Notifier>,
        hour: u32,
        top_n: i64,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            pool,
            notifier,
            hour,
            top_n,
            cancel,
        }
    }

    /// 启动定时任务：sleep 到下个 hour:00 -> 发日报 -> 每 24h 一次。
    pub fn start(&self) -> JoinHandle<()> {
        let pool = self.pool.clone();
        let notifier = self.notifier.clone();
        let hour = self.hour;
        let top_n = self.top_n;
        let cancel = self.cancel.clone();

        tokio::spawn(async move {
            let delay = next_digest_delay(Utc::now(), hour);
            info!(?delay, hour, "digest scheduler waiting for first run");
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = cancel.cancelled() => return,
            }
            loop {
                run_digest(&pool, &*notifier, top_n).await;
                tokio::select! {
                    _ = tokio::time::sleep(DAY) => {}
                    _ = cancel.cancelled() => {
                        info!("digest scheduler stopping");
                        return;
                    }
                }
            }
        })
    }
}

async fn run_digest(pool: &PgPool, notifier: &dyn Notifier, top_n: i64) {
    match generate_digest(pool, top_n).await {
        Ok(Some(report)) => {
            info!(count = report.count, "digest generated, sending");
            if let Err(e) = notifier.send(&report.title, &report.markdown).await {
                warn!(error = %e, "digest push failed");
            }
        }
        Ok(None) => info!("no new projects today, skip digest"),
        Err(e) => warn!(error = %e, "generate digest failed"),
    }
}

impl DigestScheduler {
    /// 手动触发一次日报（admin 端点用）。
    pub async fn run_once(&self) -> anyhow::Result<Option<crate::digest::DigestReport>> {
        let report = generate_digest(&self.pool, self.top_n).await?;
        if let Some(ref r) = report {
            if let Err(e) = self.notifier.send(&r.title, &r.markdown).await {
                warn!(error = %e, "manual digest push failed");
            }
        }
        Ok(report)
    }
}
