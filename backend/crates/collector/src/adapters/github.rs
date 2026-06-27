//! GitHub search/repositories adapter。

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::error::SourceError;
use domain::models::SourceKind;
use domain::source::{GithubRepoRaw, RawItem, SourceAdapter};
use reqwest::{header, StatusCode};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::rate_limit::RateLimit;
use crate::token::TokenRotator;

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    items: Vec<RepoJson>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RepoJson {
    node_id: Option<String>,
    name: String,
    full_name: String,
    html_url: String,
    description: Option<String>,
    homepage: Option<String>,
    language: Option<String>,
    topics: Option<Vec<String>>,
    stargazers_count: i64,
    forks_count: i64,
    open_issues_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub struct GithubAdapter {
    client: reqwest::Client,
    base_url: String,
    query: String,
    per_page: u32,
    max_pages: u32,
    tokens: Arc<TokenRotator>,
    limiter: Arc<dyn RateLimit>,
}

impl GithubAdapter {
    /// `query` 为 GitHub search 的 `q` 参数（如 `stars:>1000`）。
    pub fn new(
        client: reqwest::Client,
        base_url: String,
        query: String,
        per_page: u32,
        max_pages: u32,
        tokens: Arc<TokenRotator>,
        limiter: Arc<dyn RateLimit>,
    ) -> Self {
        Self {
            client,
            base_url,
            query,
            per_page: per_page.clamp(1, 100),
            max_pages,
            tokens,
            limiter,
        }
    }

    fn build_query(&self, since: Option<DateTime<Utc>>) -> String {
        match since {
            Some(t) => format!("{} pushed:>{}", self.query, t.format("%Y-%m-%d")),
            None => self.query.clone(),
        }
    }

    async fn fetch_page(
        &self,
        query: &str,
        page: u32,
    ) -> Result<Vec<RepoJson>, SourceError> {
        self.limiter.acquire().await;

        let mut req = self
            .client
            .get(format!("{}/search/repositories", self.base_url))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "openradar")
            .query(&[
                ("q", query.to_string()),
                ("sort", "stars".to_string()),
                ("order", "desc".to_string()),
                ("per_page", self.per_page.to_string()),
                ("page", page.to_string()),
            ]);
        if let Some(tok) = self.tokens.current() {
            req = req.header(header::AUTHORIZATION, format!("Bearer {tok}"));
        }

        let resp = req.send().await.map_err(|e| SourceError::Other(e.to_string()))?;
        let status = resp.status();

        if status == StatusCode::TOO_MANY_REQUESTS
            || status == StatusCode::FORBIDDEN
        {
            // 命中限流：轮换 token，交由调度器退避重试
            self.tokens.rotate();
            return Err(SourceError::RateLimited);
        }
        if !status.is_success() {
            return Err(SourceError::Other(format!("github status {status}")));
        }

        let parsed: SearchResponse =
            resp.json().await.map_err(|e| SourceError::Other(e.to_string()))?;
        Ok(parsed.items)
    }
}

#[async_trait]
impl SourceAdapter for GithubAdapter {
    fn source_kind(&self) -> SourceKind {
        SourceKind::Github
    }

    async fn fetch(
        &self,
        since: Option<DateTime<Utc>>,
        cancel: &CancellationToken,
    ) -> Result<Vec<RawItem>, SourceError> {
        let query = self.build_query(since);
        let mut out = Vec::new();
        for page in 1..=self.max_pages {
            if cancel.is_cancelled() {
                return Err(SourceError::Cancelled);
            }
            let items = self.fetch_page(&query, page).await?;
            let got = items.len();
            for r in items {
                out.push(RawItem::GithubRepo(RepoJson::into_raw(r)));
            }
            if (got as u32) < self.per_page {
                break;
            }
        }
        Ok(out)
    }
}

impl RepoJson {
    fn into_raw(self) -> GithubRepoRaw {
        let extra = serde_json::to_value(&self).unwrap_or(serde_json::Value::Null);
        GithubRepoRaw {
            full_name: self.full_name,
            name: self.name,
            description: self.description,
            html_url: self.html_url,
            homepage: self.homepage,
            language: self.language,
            topics: self.topics.unwrap_or_default(),
            stargazers_count: self.stargazers_count,
            forks_count: self.forks_count,
            open_issues_count: self.open_issues_count,
            created_at: self.created_at,
            updated_at: self.updated_at,
            node_id: self.node_id,
            extra,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limit::NoLimit;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn adapter(base_url: String) -> GithubAdapter {
        let client = reqwest::Client::new();
        GithubAdapter::new(
            client,
            base_url,
            "stars:>10".into(),
            2,
            5,
            Arc::new(TokenRotator::new(vec!["t1".into(), "t2".into()])),
            Arc::new(NoLimit),
        )
    }

    fn repo_json(full_name: &str, stars: i64) -> serde_json::Value {
        serde_json::json!({
            "node_id": "N1",
            "name": full_name.rsplit('/').next().unwrap(),
            "full_name": full_name,
            "html_url": format!("https://github.com/{full_name}"),
            "description": "desc",
            "homepage": "https://example.com",
            "language": "Rust",
            "topics": ["async"],
            "stargazers_count": stars,
            "forks_count": 10,
            "open_issues_count": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-02T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn fetch_parses_pages_until_short_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search/repositories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [repo_json("a/b", 100), repo_json("c/d", 90)]
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/search/repositories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [repo_json("e/f", 80)]
            })))
            .mount(&server)
            .await;

        let a = adapter(server.uri());
        let cancel = CancellationToken::new();
        let items = a.fetch(None, &cancel).await.unwrap();
        assert_eq!(items.len(), 3);
        match &items[0] {
            RawItem::GithubRepo(g) => {
                assert_eq!(g.full_name, "a/b");
                assert_eq!(g.stargazers_count, 100);
            }
            _ => panic!("expected github repo"),
        }
    }

    #[tokio::test]
    async fn rate_limit_returns_error_and_rotates() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search/repositories"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let tokens = Arc::new(TokenRotator::new(vec!["t1".into(), "t2".into()]));
        let a = GithubAdapter::new(
            reqwest::Client::new(),
            server.uri(),
            "stars:>10".into(),
            10,
            5,
            tokens.clone(),
            Arc::new(NoLimit),
        );
        let cancel = CancellationToken::new();
        let err = a.fetch(None, &cancel).await.unwrap_err();
        assert!(matches!(err, SourceError::RateLimited));
        // 429 后应已轮换
        assert_eq!(tokens.current().as_deref(), Some("t2"));
    }

    #[tokio::test]
    async fn cancellation_aborts_fetch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search/repositories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [repo_json("a/b", 1), repo_json("c/d", 2)]
            })))
            .mount(&server)
            .await;

        let a = adapter(server.uri());
        let cancel = CancellationToken::new();
        cancel.cancel();
        let err = a.fetch(None, &cancel).await.unwrap_err();
        assert!(matches!(err, SourceError::Cancelled));
    }
}
