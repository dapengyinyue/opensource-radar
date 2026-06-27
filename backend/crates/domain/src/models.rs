//! 领域模型。Phase-1 MVP 的核心数据结构。

use serde::{Deserialize, Serialize};

/// 采集源类型。映射到 PG 的 `source_kind` 枚举（storage 层做映射）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Github,
    Hackernews,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Github => "github",
            SourceKind::Hackernews => "hackernews",
        }
    }
}

/// 归一后的项目记录：去重键 + 跨源归一字段。upsert 入库的输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRecord {
    pub dedup_key: String,
    pub name: String,
    pub full_name: Option<String>,
    pub description: Option<String>,
    pub repo_url: Option<String>,
    pub homepage_url: Option<String>,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub stars: Option<i64>,
    pub forks: Option<i64>,
    pub open_issues: Option<i64>,
    pub hn_points: Option<i64>,
    pub hn_comment_count: Option<i64>,
    pub github_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub github_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_activity_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source_kind: SourceKind,
    /// 源特有、未归一字段，落 projects.metadata。
    pub metadata: serde_json::Value,
}
