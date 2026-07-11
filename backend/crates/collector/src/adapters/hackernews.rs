//! HackerNews Algolia adapter。
//! API: GET {base}/search?tags=story&numericFilters=...&hitsPerPage=N&page=P

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::error::SourceError;
use domain::models::SourceKind;
use domain::source::{HnStoryRaw, RawItem, SourceAdapter};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::rate_limit::RateLimit;

#[derive(Debug, Clone, Deserialize)]
struct HitsResponse {
    hits: Vec<HitJson>,
    #[serde(default)]
    nb_pages: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct HitJson {
    #[serde(rename = "objectID")]
    object_id: String,
    #[serde(default)]
    title: Option<String>,
    /// 故事指向的外链（Ask/Show HN 无外链时为 None）
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    points: Option<i64>,
    #[serde(default)]
    num_comments: Option<i64>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
}

pub struct HnAdapter {
    client: reqwest::Client,
    base_url: String,
    tags: String,
    numeric_filter: Option<String>,
    hits_per_page: u32,
    max_pages: u32,
    limiter: Arc<dyn RateLimit>,
}

impl HnAdapter {
    pub fn new(
        client: reqwest::Client,
        base_url: String,
        tags: String,
        numeric_filter: Option<String>,
        hits_per_page: u32,
        max_pages: u32,
        limiter: Arc<dyn RateLimit>,
    ) -> Self {
        Self {
            client,
            base_url,
            tags,
            numeric_filter,
            hits_per_page: hits_per_page.clamp(1, 50),
            max_pages,
            limiter,
        }
    }

    async fn fetch_page(&self, page: u32) -> Result<HitsResponse, SourceError> {
        self.limiter.acquire().await;

        let mut q: Vec<(&str, String)> = vec![
            ("tags", self.tags.clone()),
            ("hitsPerPage", self.hits_per_page.to_string()),
            ("page", page.to_string()),
        ];
        if let Some(f) = &self.numeric_filter {
            q.push(("numericFilters", f.clone()));
        }

        let resp = self
            .client
            .get(format!("{}/search", self.base_url))
            .header("User-Agent", "openradar")
            .query(&q)
            .send()
            .await
            .map_err(|e| SourceError::Other(e.to_string()))?;
        let status = resp.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(SourceError::RateLimited);
        }
        if !status.is_success() {
            return Err(SourceError::Other(format!("hn status {status}")));
        }

        resp.json()
            .await
            .map_err(|e| SourceError::Other(e.to_string()))
    }
}

#[async_trait]
impl SourceAdapter for HnAdapter {
    fn source_kind(&self) -> SourceKind {
        SourceKind::Hackernews
    }

    async fn fetch(
        &self,
        _since: Option<DateTime<Utc>>,
        cancel: &CancellationToken,
    ) -> Result<Vec<RawItem>, SourceError> {
        let mut out = Vec::new();
        for page in 0..self.max_pages {
            if cancel.is_cancelled() {
                return Err(SourceError::Cancelled);
            }
            let resp = self.fetch_page(page).await?;
            let got = resp.hits.len();
            for h in resp.hits {
                out.push(RawItem::HnStory(HitJson::into_raw(h)));
            }
            let last_page = resp.nb_pages != 0 && page + 1 >= resp.nb_pages;
            if (got as u32) < self.hits_per_page || last_page {
                break;
            }
        }
        Ok(out)
    }
}

impl HitJson {
    fn into_raw(self) -> HnStoryRaw {
        let extra = serde_json::to_value(&self).unwrap_or(serde_json::Value::Null);
        let object_id = self.object_id.clone();
        HnStoryRaw {
            hn_url: format!("https://news.ycombinator.com/item?id={object_id}"),
            object_id: self.object_id,
            linked_url: self.url,
            title: self.title.unwrap_or_default(),
            author: self.author,
            points: self.points,
            comment_count: self.num_comments,
            posted_at: self.created_at,
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

    fn adapter(base_url: String) -> HnAdapter {
        HnAdapter::new(
            reqwest::Client::new(),
            base_url,
            "story".into(),
            Some("points>=50".into()),
            2,
            5,
            Arc::new(NoLimit),
        )
    }

    fn hit(id: &str, url: Option<&str>) -> serde_json::Value {
        serde_json::json!({
            "objectID": id,
            "title": format!("Story {id}"),
            "url": url,
            "author": "bob",
            "points": 120,
            "num_comments": 45,
            "created_at": "2026-01-01T00:00:00.000Z"
        })
    }

    #[tokio::test]
    async fn fetch_parses_hits_and_stops_at_nb_pages() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "hits": [hit("1", Some("https://github.com/tokio-rs/tokio")), hit("2", Some("https://crates.io/crates/axum"))],
                "nb_pages": 1
            })))
            .mount(&server)
            .await;

        let a = adapter(server.uri());
        let cancel = CancellationToken::new();
        let items = a.fetch(None, &cancel).await.unwrap();
        assert_eq!(items.len(), 2);
        match &items[0] {
            RawItem::HnStory(h) => {
                assert_eq!(h.object_id, "1");
                assert_eq!(
                    h.linked_url.as_deref(),
                    Some("https://github.com/tokio-rs/tokio")
                );
                assert_eq!(h.points, Some(120));
                assert_eq!(h.comment_count, Some(45));
            }
            _ => panic!("expected hn story"),
        }
    }

    #[tokio::test]
    async fn rate_limit_returns_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let a = adapter(server.uri());
        let cancel = CancellationToken::new();
        let err = a.fetch(None, &cancel).await.unwrap_err();
        assert!(matches!(err, SourceError::RateLimited));
    }

    #[tokio::test]
    async fn ask_hn_without_url_still_collected() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "hits": [hit("9", None)],
                "nb_pages": 1
            })))
            .mount(&server)
            .await;

        let a = adapter(server.uri());
        let cancel = CancellationToken::new();
        let items = a.fetch(None, &cancel).await.unwrap();
        match &items[0] {
            RawItem::HnStory(h) => assert!(h.linked_url.is_none()),
            _ => panic!(),
        }
    }
}
