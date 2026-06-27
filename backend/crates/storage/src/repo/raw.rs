use anyhow::Result;
use chrono::Utc;
use domain::source::{GithubRepoRaw, HnStoryRaw};
use sqlx::PgExecutor;

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
