//! scheduler 集成测试：依赖本地 PG（DATABASE_URL_TEST）。

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::error::SourceError;
use domain::models::SourceKind;
use domain::source::{GithubRepoRaw, HnStoryRaw, RawItem, SourceAdapter};
use collector::scheduler::{Collector, ScheduledAdapter};
use sqlx::PgPool;
use storage::pool;
use tokio_util::sync::CancellationToken;

static SERIALIZE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> PgPool {
    let url = std::env::var("DATABASE_URL_TEST")
        .unwrap_or_else(|_| "postgres://localhost:5432/openradar_test".into());
    let pool = pool::create_pool(&url).await.expect("connect");
    pool::run_migrations(&pool).await.expect("migrate");
    sqlx::query(
        "TRUNCATE projects, raw_github_repos, raw_hn_stories, project_snapshots RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .expect("truncate");
    pool
}

fn ts() -> DateTime<Utc> {
    "2026-01-01T00:00:00Z".parse().unwrap()
}

fn gh() -> GithubRepoRaw {
    GithubRepoRaw {
        full_name: "tokio-rs/tokio".into(),
        name: "tokio".into(),
        description: Some("rt".into()),
        html_url: "https://github.com/tokio-rs/tokio".into(),
        homepage: None,
        language: Some("Rust".into()),
        topics: vec![],
        stargazers_count: 100,
        forks_count: 10,
        open_issues_count: 1,
        created_at: ts(),
        updated_at: ts(),
        node_id: None,
        extra: serde_json::json!({}),
    }
}

fn hn() -> HnStoryRaw {
    HnStoryRaw {
        object_id: "42".into(),
        hn_url: "https://news.ycombinator.com/item?id=42".into(),
        linked_url: Some("https://github.com/tokio-rs/tokio".into()),
        title: "Tokio".into(),
        author: None,
        points: Some(50),
        comment_count: Some(5),
        posted_at: Some(ts()),
        extra: serde_json::json!({}),
    }
}

struct MockAdapter {
    items: Vec<RawItem>,
    kind: SourceKind,
}

#[async_trait]
impl SourceAdapter for MockAdapter {
    fn source_kind(&self) -> SourceKind {
        self.kind
    }
    async fn fetch(
        &self,
        _: Option<DateTime<Utc>>,
        _: &CancellationToken,
    ) -> Result<Vec<RawItem>, SourceError> {
        Ok(self.items.clone())
    }
}

#[tokio::test]
async fn run_once_persists_items() {
    let _g = SERIALIZE.lock().await;
    let pool = setup().await;
    let cancel = CancellationToken::new();
    let mut collector = Collector::new(pool.clone(), cancel);
    collector.register(ScheduledAdapter {
        kind: SourceKind::Github,
        name: "mock".into(),
        adapter: Arc::new(MockAdapter {
            items: vec![RawItem::GithubRepo(gh()), RawItem::HnStory(hn())],
            kind: SourceKind::Github,
        }),
        period: Duration::from_secs(3600),
    });

    let count = collector.run_once(SourceKind::Github).await.unwrap();
    assert_eq!(count, 2);

    // 两个 raw item 指向同一 repo → 合并为 1 个 project
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM projects")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
    let p = storage::repo::project::get(&pool, 1).await.unwrap().unwrap();
    assert!(p.source_kinds.contains(&"github".to_string()));
    assert!(p.source_kinds.contains(&"hackernews".to_string()));
}

#[tokio::test]
async fn run_once_unregistered_source_errors() {
    let _g = SERIALIZE.lock().await;
    let pool = setup().await;
    let cancel = CancellationToken::new();
    let collector = Collector::new(pool, cancel);
    let err = collector.run_once(SourceKind::Hackernews).await;
    assert!(err.is_err());
}
