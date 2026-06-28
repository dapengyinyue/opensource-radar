//! storage 集成测试：依赖本地 PG（DATABASE_URL_TEST，默认 openradar_test）。

use chrono::Utc;
use domain::source::{GithubRepoRaw, HnStoryRaw, RawItem};
use sqlx::PgPool;
use storage::pool;
use storage::repo::{project, snapshot};

/// 所有测试共用同一个 openradar_test 库，必须串行执行以免互相 truncate。
static SERIALIZE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> PgPool {
    let url = std::env::var("DATABASE_URL_TEST")
        .unwrap_or_else(|_| "postgres://localhost:5432/openradar_test".into());
    let pool = pool::create_pool(&url).await.expect("connect pg");
    pool::run_migrations(&pool).await.expect("migrate");
    sqlx::query(
        "TRUNCATE projects, raw_github_repos, raw_hn_stories, project_snapshots RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .expect("truncate");
    pool
}

fn ts() -> chrono::DateTime<chrono::Utc> {
    "2026-01-01T00:00:00Z".parse().unwrap()
}

fn gh() -> GithubRepoRaw {
    GithubRepoRaw {
        full_name: "Tokio-Rs/tokio".into(),
        name: "tokio".into(),
        description: Some("async runtime".into()),
        html_url: "https://github.com/tokio-rs/tokio".into(),
        homepage: Some("https://tokio.rs".into()),
        language: Some("Rust".into()),
        topics: vec!["async".into(), "runtime".into()],
        stargazers_count: 27000,
        forks_count: 3000,
        open_issues_count: 400,
        created_at: ts(),
        updated_at: ts(),
        node_id: Some("N1".into()),
        extra: serde_json::json!({}),
    }
}

fn hn(linked: Option<&str>, object_id: &str) -> HnStoryRaw {
    HnStoryRaw {
        object_id: object_id.into(),
        hn_url: format!("https://news.ycombinator.com/item?id={object_id}"),
        linked_url: linked.map(String::from),
        title: "Tokio on HN".into(),
        author: Some("alice".into()),
        points: Some(120),
        comment_count: Some(45),
        posted_at: Some(ts()),
        extra: serde_json::json!({}),
    }
}

#[tokio::test]
async fn github_then_hn_merges_into_one_project() {
    let _g = SERIALIZE.lock().await;
    let pool = setup().await;

    let id1 = storage::repo::persist_raw_item(&pool, &RawItem::GithubRepo(gh()))
        .await
        .expect("persist github");
    let id2 = storage::repo::persist_raw_item(
        &pool,
        &RawItem::HnStory(hn(Some("https://github.com/tokio-rs/tokio/issues/1"), "111")),
    )
    .await
    .expect("persist hn");

    assert_eq!(id1, id2, "HN linking same repo should merge into same project");

    let p = project::get(&pool, id1).await.unwrap().expect("project exists");
    assert_eq!(p.dedup_key, "gh:tokio-rs/tokio");
    assert!(p.source_kinds.contains(&"github".to_string()));
    assert!(p.source_kinds.contains(&"hackernews".to_string()));
    assert_eq!(p.stars, Some(27000), "github stars preserved");
    assert_eq!(p.hn_points, Some(120), "hn points merged in");
    assert_eq!(p.language.as_deref(), Some("Rust"));

    // 两次采集 → 两条快照
    let snaps = snapshot::list_snapshots(&pool, id1, 10).await.unwrap();
    assert_eq!(snaps.len(), 2);
}

#[tokio::test]
async fn hn_non_github_link_is_separate_project() {
    let _g = SERIALIZE.lock().await;
    let pool = setup().await;

    let _gh_id = storage::repo::persist_raw_item(&pool, &RawItem::GithubRepo(gh()))
        .await
        .unwrap();
    let ext_id = storage::repo::persist_raw_item(
        &pool,
        &RawItem::HnStory(hn(Some("https://crates.io/crates/axum"), "222")),
    )
    .await
    .unwrap();

    let p = project::get(&pool, ext_id).await.unwrap().unwrap();
    assert_eq!(p.dedup_key, "url:crates.io/crates/axum");
    assert!(p.source_kinds.contains(&"hackernews".to_string()));
    assert!(!p.source_kinds.contains(&"github".to_string()));
    assert!(p.stars.is_none());
}

#[tokio::test]
async fn search_by_query_matches_name_full_name_description() {
    let _g = SERIALIZE.lock().await;
    let pool = setup().await;
    // gh: name="tokio", full_name="tokio-rs/tokio", description="async runtime"
    storage::repo::persist_raw_item(&pool, &RawItem::GithubRepo(gh()))
        .await
        .unwrap();
    // hn: 非 github 外链，name/description = title = "Tokio on HN"
    storage::repo::persist_raw_item(
        &pool,
        &RawItem::HnStory(hn(Some("https://crates.io/crates/axum"), "333")),
    )
    .await
    .unwrap();

    let mk = |q: &str| project::ProjectFilter {
        q: Some(q.into()),
        per_page: 100,
        ..Default::default()
    };

    // "tokio" 命中两条：gh name=tokio，hn name="Tokio on HN"（ILIKE 大小写不敏感）
    let hits = project::list(&pool, &mk("tokio")).await.unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(project::count(&pool, &mk("tokio")).await.unwrap(), 2);

    // "runtime" 仅命中 gh（description="async runtime"）
    let hits = project::list(&pool, &mk("runtime")).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, "tokio");

    // 无匹配
    assert_eq!(project::list(&pool, &mk("nonexistent-zzz")).await.unwrap().len(), 0);
    assert_eq!(project::count(&pool, &mk("nonexistent-zzz")).await.unwrap(), 0);
}

#[tokio::test]
async fn list_filters_and_pagination() {
    let _g = SERIALIZE.lock().await;
    let pool = setup().await;
    storage::repo::persist_raw_item(&pool, &RawItem::GithubRepo(gh()))
        .await
        .unwrap();
    storage::repo::persist_raw_item(
        &pool,
        &RawItem::HnStory(hn(Some("https://crates.io/crates/axum"), "333")),
    )
    .await
    .unwrap();

    let all = project::list(
        &pool,
        &project::ProjectFilter {
            per_page: 100,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(all.len(), 2);

    let rust_only = project::list(
        &pool,
        &project::ProjectFilter {
            language: Some("Rust".into()),
            per_page: 100,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rust_only.len(), 1);
    assert_eq!(rust_only[0].name, "tokio");

    let hn_only = project::list(
        &pool,
        &project::ProjectFilter {
            source: Some("hackernews".into()),
            per_page: 100,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(hn_only.len(), 1);
    assert_eq!(hn_only[0].dedup_key, "url:crates.io/crates/axum");
}

#[tokio::test]
async fn upsert_updates_metrics_on_rescan() {
    let _g = SERIALIZE.lock().await;
    let pool = setup().await;
    let id = storage::repo::persist_raw_item(&pool, &RawItem::GithubRepo(gh()))
        .await
        .unwrap();

    let mut g2 = gh();
    g2.stargazers_count = 28000;
    let id_again = storage::repo::persist_raw_item(&pool, &RawItem::GithubRepo(g2))
        .await
        .unwrap();
    assert_eq!(id, id_again);

    let p = project::get(&pool, id).await.unwrap().unwrap();
    assert_eq!(p.stars, Some(28000));
    let snaps = snapshot::list_snapshots(&pool, id, 10).await.unwrap();
    assert_eq!(snaps.len(), 2);
    // ordered desc: 最新在前
    assert_eq!(snaps[0].stars, Some(28000));
    assert_eq!(snaps[1].stars, Some(27000));
    let _ = Utc::now(); // ensure chrono Utc linked
}
