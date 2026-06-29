//! API 集成测试：tower oneshot + 本地 PG（DATABASE_URL_TEST）。

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use collector::scheduler::Collector;
use domain::source::{GithubRepoRaw, HnStoryRaw, RawItem};
use server::app::{router, AppState};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

static SERIALIZE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn ts() -> chrono::DateTime<chrono::Utc> {
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
        topics: vec!["async".into()],
        stargazers_count: 100,
        forks_count: 10,
        open_issues_count: 1,
        created_at: ts(),
        updated_at: ts(),
        node_id: None,
        extra: serde_json::json!({}),
    }
}

fn hn_ext() -> HnStoryRaw {
    HnStoryRaw {
        object_id: "77".into(),
        hn_url: "https://news.ycombinator.com/item?id=77".into(),
        linked_url: Some("https://crates.io/crates/axum".into()),
        title: "Axum".into(),
        author: None,
        points: Some(60),
        comment_count: Some(4),
        posted_at: Some(ts()),
        extra: serde_json::json!({}),
    }
}

async fn setup() -> (AppState, sqlx::PgPool) {
    let url = std::env::var("DATABASE_URL_TEST")
        .unwrap_or_else(|_| "postgres://localhost:5432/openradar_test".into());
    let pool = storage::pool::create_pool(&url).await.unwrap();
    storage::pool::run_migrations(&pool).await.unwrap();
    sqlx::query(
        "TRUNCATE projects, raw_github_repos, raw_hn_stories, project_snapshots RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();
    storage::repo::persist_raw_item(&pool, &RawItem::GithubRepo(gh()))
        .await
        .unwrap();
    storage::repo::persist_raw_item(&pool, &RawItem::HnStory(hn_ext()))
        .await
        .unwrap();

    let collector = Arc::new(Collector::new(pool.clone(), CancellationToken::new()));
    let state = AppState {
        pool: pool.clone(),
        collector,
        admin_token: "secret".into(),
    };
    (state, pool)
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn list_and_detail() {
    let _g = SERIALIZE.lock().await;
    let (state, _pool) = setup().await;
    let app = router(state);

    let resp = app
        .clone()
        .oneshot(Request::get("/api/v1/projects").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["total"], 2);
    assert_eq!(v["data"].as_array().unwrap().len(), 2);

    let id = v["data"][0]["id"].as_i64().unwrap();
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/projects/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = body_json(resp).await;
    assert!(detail["dedup_key"].is_string());
}

#[tokio::test]
async fn list_filters_by_query() {
    let _g = SERIALIZE.lock().await;
    let (state, _pool) = setup().await;
    let app = router(state);

    // gh: name="tokio"; hn_ext: name="Axum"
    let resp = app
        .clone()
        .oneshot(
            Request::get("/api/v1/projects?q=tokio")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(resp).await;
    assert_eq!(v["total"], 1);
    assert_eq!(v["data"][0]["name"], "tokio");

    // 大小写不敏感 + 命中 hn_ext 的 name "Axum"
    let resp = app
        .oneshot(
            Request::get("/api/v1/projects?q=axum")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(resp).await;
    assert_eq!(v["total"], 1);
    assert_eq!(v["data"][0]["name"], "Axum");
}

#[tokio::test]
async fn list_filters_by_language() {
    let _g = SERIALIZE.lock().await;
    let (state, _pool) = setup().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/projects?language=Rust")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(resp).await;
    assert_eq!(v["total"], 1);
    assert_eq!(v["data"][0]["name"], "tokio");
}

#[tokio::test]
async fn detail_404() {
    let _g = SERIALIZE.lock().await;
    let (state, _pool) = setup().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/projects/999999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn facets_and_snapshots() {
    let _g = SERIALIZE.lock().await;
    let (state, pool) = setup().await;
    let app = router(state);

    let resp = app
        .clone()
        .oneshot(Request::get("/api/v1/languages").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let langs = body_json(resp).await;
    assert!(langs.as_array().unwrap().iter().any(|f| f["key"] == "Rust"));

    // tokio project id
    let id: i64 = sqlx::query_scalar("SELECT id FROM projects WHERE dedup_key = 'gh:tokio-rs/tokio'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/projects/{id}/snapshots"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let snaps = body_json(resp).await;
    assert_eq!(snaps.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn admin_auth_and_validation() {
    let _g = SERIALIZE.lock().await;
    let (state, _pool) = setup().await;
    let app = router(state);

    // 无 token → 401
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/v1/admin/collect/github")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 错误 token → 401
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/v1/admin/collect/github")
                .header("X-Admin-Token", "wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 正确 token + 未知源 → 400
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/v1/admin/collect/reddit")
                .header("X-Admin-Token", "secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 正确 token + 合法源 → 202
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/collect/github")
                .header("X-Admin-Token", "secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn project_sources_returns_github_and_hn() {
    let _g = SERIALIZE.lock().await;
    let (state, pool) = setup().await;

    // 在 tokio project 上追加一条指向它的 HN 故事 → 合并项目（github + hackernews）
    let merged = HnStoryRaw {
        object_id: "888".into(),
        hn_url: "https://news.ycombinator.com/item?id=888".into(),
        linked_url: Some("https://github.com/tokio-rs/tokio".into()),
        title: "Tokio discussion".into(),
        author: Some("bob".into()),
        points: Some(80),
        comment_count: Some(10),
        posted_at: Some(ts()),
        extra: serde_json::json!({}),
    };
    storage::repo::persist_raw_item(&pool, &RawItem::HnStory(merged))
        .await
        .unwrap();

    let app = router(state);
    let tokio_id: i64 = sqlx::query_scalar(
        "SELECT id FROM projects WHERE dedup_key = 'gh:tokio-rs/tokio'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/projects/{tokio_id}/sources"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    // github repo 明细
    assert_eq!(v["github"]["full_name"], "tokio-rs/tokio");
    // 至少一条 HN 故事（888），按 points desc 排序
    let hn = v["hackernews"].as_array().unwrap();
    assert!(!hn.is_empty());
    assert_eq!(hn[0]["object_id"], "888");
    assert_eq!(hn[0]["hn_url"], "https://news.ycombinator.com/item?id=888");
    assert_eq!(hn[0]["author"], "bob");
}

#[tokio::test]
async fn project_sources_404_for_missing() {
    let _g = SERIALIZE.lock().await;
    let (state, _pool) = setup().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/projects/999999/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
