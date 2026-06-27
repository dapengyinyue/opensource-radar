pub mod api;
pub mod app;
pub mod config;
pub mod error;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use collector::adapters::{github::GithubAdapter, hackernews::HnAdapter};
use collector::rate_limit::{GovernorLimiter, RateLimit};
use collector::scheduler::{Collector, ScheduledAdapter};
use collector::token::TokenRotator;
use domain::models::SourceKind;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

pub async fn run() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = config::Settings::from_env()?;
    tracing::info!(bind = %cfg.bind_addr, "starting openradar server");

    let pool = storage::pool::create_pool(&cfg.database_url).await?;
    storage::pool::run_migrations(&pool).await?;
    tracing::info!("database migrated");

    let cancel = CancellationToken::new();
    let client = reqwest::Client::builder().gzip(true).build()?;

    let gh_limiter: Arc<dyn RateLimit> = Arc::new(GovernorLimiter::per_second(1));
    let hn_limiter: Arc<dyn RateLimit> = Arc::new(GovernorLimiter::per_second(5));
    let tokens = Arc::new(TokenRotator::new(cfg.github_tokens.clone()));

    let github = GithubAdapter::new(
        client.clone(),
        "https://api.github.com".into(),
        "stars:>50".into(),
        100,
        3,
        tokens.clone(),
        gh_limiter,
    );
    let hn = HnAdapter::new(
        client,
        "https://hn.algolia.com/api/v1".into(),
        "story".into(),
        // HN Algolia 不允许 points 作为 numericFilter；/search 默认按热度排序，无需过滤
        None,
        50,
        2,
        hn_limiter,
    );

    let mut collector = Collector::new(pool.clone(), cancel.clone());
    collector.register(ScheduledAdapter {
        kind: SourceKind::Github,
        name: "github".into(),
        adapter: Arc::new(github),
        period: Duration::from_secs(cfg.schedule_github_secs),
    });
    collector.register(ScheduledAdapter {
        kind: SourceKind::Hackernews,
        name: "hackernews".into(),
        adapter: Arc::new(hn),
        period: Duration::from_secs(cfg.schedule_hn_secs),
    });
    let collector = Arc::new(collector);
    collector.start();

    let state = app::AppState {
        pool: pool.clone(),
        collector: collector.clone(),
        admin_token: cfg.admin_token.clone(),
    };
    let app = app::router(state);

    let addr: SocketAddr = cfg.bind_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    tracing::info!("shutting down collector...");
    collector.shutdown().await;
    tracing::info!("shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("install ctrl_c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
