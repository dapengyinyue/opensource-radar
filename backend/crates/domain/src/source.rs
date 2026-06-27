//! SourceAdapter trait 与采集源原始数据结构。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::error::SourceError;
use crate::models::SourceKind;

/// GitHub repo 原始字段（API 响应选取后）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubRepoRaw {
    /// "owner/repo"
    pub full_name: String,
    pub name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub homepage: Option<String>,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub stargazers_count: i64,
    pub forks_count: i64,
    pub open_issues_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub node_id: Option<String>,
    /// 未归一的源特有字段，落 metadata。
    pub extra: serde_json::Value,
}

/// HackerNews 故事原始字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnStoryRaw {
    /// Algolia objectID
    pub object_id: String,
    /// news.ycombinator.com/item?id=...
    pub hn_url: String,
    /// 故事指向的外链（Ask/Show HN 无外链时为 None）
    pub linked_url: Option<String>,
    pub title: String,
    pub author: Option<String>,
    pub points: Option<i64>,
    pub comment_count: Option<i64>,
    pub posted_at: Option<DateTime<Utc>>,
    pub extra: serde_json::Value,
}

/// 采集器产出的原始项。
#[derive(Debug, Clone)]
pub enum RawItem {
    GithubRepo(GithubRepoRaw),
    HnStory(HnStoryRaw),
}

/// 多源采集统一接口。新增源只需实现本 trait 并在调度器注册。
#[async_trait]
pub trait SourceAdapter: Send + Sync {
    fn source_kind(&self) -> SourceKind;
    /// 拉取一批原始项。`since` 用于支持增量的源；不支持则忽略。
    async fn fetch(
        &self,
        since: Option<DateTime<Utc>>,
        cancel: &CancellationToken,
    ) -> Result<Vec<RawItem>, SourceError>;
}
