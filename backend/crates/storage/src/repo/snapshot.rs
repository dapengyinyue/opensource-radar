use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: i64,
    pub project_id: i64,
    pub stars: Option<i64>,
    pub hn_points: Option<i64>,
    pub captured_at: DateTime<Utc>,
}

pub async fn write_snapshot<'e, E>(
    exec: E,
    project_id: i64,
    stars: Option<i64>,
    hn_points: Option<i64>,
) -> Result<()>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        "INSERT INTO project_snapshots (project_id, stars, hn_points, captured_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(project_id)
    .bind(stars)
    .bind(hn_points)
    .bind(Utc::now())
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn list_snapshots<'e, E>(exec: E, project_id: i64, limit: i64) -> Result<Vec<Snapshot>>
where
    E: PgExecutor<'e>,
{
    let rows = sqlx::query_as::<_, Snapshot>(
        "SELECT id, project_id, stars, hn_points, captured_at \
         FROM project_snapshots WHERE project_id = $1 \
         ORDER BY captured_at DESC LIMIT $2",
    )
    .bind(project_id)
    .bind(limit)
    .fetch_all(exec)
    .await?;
    Ok(rows)
}
