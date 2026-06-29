use anyhow::Result;
use chrono::{DateTime, Utc};
use domain::source::{GithubRepoRaw, HnStoryRaw};
use serde::Serialize;
use sqlx::{FromRow, PgExecutor, PgPool};

pub async fn upsert_github<'e, E>(exec: E, project_id: i64, g: &GithubRepoRaw) -> Result<()>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        "INSERT INTO raw_github_repos (project_id, full_name, node_id, payload, collected_at) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (full_name) DO UPDATE SET \
           project_id = EXCLUDED.project_id, node_id = EXCLUDED.node_id, \
           payload = EXCLUDED.payload, collected_at = EXCLUDED.collected_at",
    )
    .bind(project_id)
    .bind(g.full_name.to_ascii_lowercase())
    .bind(&g.node_id)
    .bind(&g.extra)
    .bind(Utc::now())
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn upsert_hn<'e, E>(exec: E, project_id: i64, h: &HnStoryRaw) -> Result<()>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        "INSERT INTO raw_hn_stories \
           (project_id, object_id, hn_url, linked_url, author, points, comment_count, posted_at, payload, collected_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) \
         ON CONFLICT (object_id) DO UPDATE SET \
           project_id = EXCLUDED.project_id, hn_url = EXCLUDED.hn_url, linked_url = EXCLUDED.linked_url, \
           author = EXCLUDED.author, points = EXCLUDED.points, comment_count = EXCLUDED.comment_count, \
           posted_at = EXCLUDED.posted_at, payload = EXCLUDED.payload, collected_at = EXCLUDED.collected_at",
    )
    .bind(project_id)
    .bind(&h.object_id)
    .bind(&h.hn_url)
    .bind(&h.linked_url)
    .bind(&h.author)
    .bind(h.points)
    .bind(h.comment_count)
    .bind(h.posted_at)
    .bind(&h.extra)
    .bind(Utc::now())
    .execute(exec)
    .await?;
    Ok(())
}

/// 项目的 HN 故事原始记录（详情页 HN 讨论外链用）。
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct HnStoryRecord {
    pub object_id: String,
    pub hn_url: String,
    pub linked_url: Option<String>,
    pub author: Option<String>,
    pub points: Option<i64>,
    pub comment_count: Option<i64>,
    pub posted_at: Option<DateTime<Utc>>,
}

/// 项目的 GitHub repo 原始记录。
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct GithubRepoRecord {
    pub full_name: String,
    pub node_id: Option<String>,
}

/// 详情页「来源明细」聚合：一个项目被哪些源的哪些原始记录观察到。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectSources {
    pub github: Option<GithubRepoRecord>,
    pub hackernews: Vec<HnStoryRecord>,
}

pub async fn list_project_sources(pool: &PgPool, project_id: i64) -> Result<ProjectSources> {
    let github = sqlx::query_as::<_, GithubRepoRecord>(
        "SELECT full_name, node_id FROM raw_github_repos WHERE project_id = $1 LIMIT 1",
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    let hackernews = sqlx::query_as::<_, HnStoryRecord>(
        "SELECT object_id, hn_url, linked_url, author, points, comment_count, posted_at \
         FROM raw_hn_stories WHERE project_id = $1 ORDER BY points DESC NULLS LAST",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(ProjectSources { github, hackernews })
}
