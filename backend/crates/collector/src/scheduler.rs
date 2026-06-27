//! 调度器：周期触发各源采集，限流/重试/优雅关闭。
//! 每源一个独立 tokio 任务，select! 在 interval tick 与 cancellation 之间竞争。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use domain::error::SourceError;
use domain::models::SourceKind;
use domain::source::SourceAdapter;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

const MAX_ATTEMPTS: u32 = 3;
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);

pub struct ScheduledAdapter {
    pub kind: SourceKind,
    pub name: String,
    pub adapter: Arc<dyn SourceAdapter>,
    pub period: Duration,
}

pub struct Collector {
    entries: Vec<ScheduledAdapter>,
    pool: PgPool,
    cancel: CancellationToken,
    handles: std::sync::Mutex<Vec<JoinHandle<()>>>,
}

impl Collector {
    pub fn new(pool: PgPool, cancel: CancellationToken) -> Self {
        Self {
            entries: Vec::new(),
            pool,
            cancel,
            handles: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn register(&mut self, entry: ScheduledAdapter) {
        self.entries.push(entry);
    }

    /// 启动所有源的周期采集任务。
    pub fn start(&self) {
        for entry in &self.entries {
            let kind = entry.kind;
            let name = entry.name.clone();
            let adapter = entry.adapter.clone();
            let period = entry.period;
            let pool = self.pool.clone();
            let cancel = self.cancel.clone();
            info!(source = %name, kind = ?kind, "registered collector task");

            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(period);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Err(e) = run_once(&name, &*adapter, &pool, &cancel).await {
                                warn!(source = %name, error = %e, "collect run failed");
                            }
                        }
                        _ = cancel.cancelled() => {
                            info!(source = %name, "collector task stopping");
                            break;
                        }
                    }
                }
            });
            self.handles.lock().unwrap().push(handle);
        }
    }

    /// 手动触发某源采集一次（admin 端点用）。返回入库条数。
    pub async fn run_once(&self, kind: SourceKind) -> Result<usize> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.kind == kind)
            .ok_or_else(|| anyhow::anyhow!("source {kind:?} not registered"))?;
        run_once(&entry.name, &*entry.adapter, &self.pool, &self.cancel).await
    }

    /// 优雅关闭：取消调度，等待在途任务（带超时）。
    pub async fn shutdown(&self) {
        self.cancel.cancel();
        let handles: Vec<_> = self.handles.lock().unwrap().drain(..).collect();
        for h in handles {
            let _ = tokio::time::timeout(Duration::from_secs(30), h).await;
        }
    }
}

async fn run_once(
    name: &str,
    adapter: &dyn SourceAdapter,
    pool: &PgPool,
    cancel: &CancellationToken,
) -> Result<usize> {
    let items = fetch_with_retry(name, adapter, None, cancel).await?;
    let mut count = 0usize;
    for item in items {
        if cancel.is_cancelled() {
            break;
        }
        if let Err(e) = storage::repo::persist_raw_item(pool, &item).await {
            warn!(source = %name, error = %e, "persist item failed");
            continue;
        }
        count += 1;
    }
    info!(source = %name, persisted = count, "collect run done");
    Ok(count)
}

async fn fetch_with_retry(
    name: &str,
    adapter: &dyn SourceAdapter,
    since: Option<chrono::DateTime<chrono::Utc>>,
    cancel: &CancellationToken,
) -> Result<Vec<domain::source::RawItem>> {
    let mut backoff = INITIAL_BACKOFF;
    for attempt in 0..MAX_ATTEMPTS {
        if cancel.is_cancelled() {
            anyhow::bail!("cancelled");
        }
        match adapter.fetch(since, cancel).await {
            Ok(v) => return Ok(v),
            Err(SourceError::Cancelled) => anyhow::bail!("cancelled"),
            Err(e) => {
                if attempt + 1 == MAX_ATTEMPTS {
                    anyhow::bail!(format!("fetch {name} failed after {MAX_ATTEMPTS} attempts: {e}"));
                }
                warn!(source = %name, attempt, error = %e, "fetch failed, retrying");
                tokio::select! {
                    _ = tokio::time::sleep(backoff) => {}
                    _ = cancel.cancelled() => anyhow::bail!("cancelled"),
                }
                backoff *= 2;
            }
        }
    }
    unreachable!()
}
