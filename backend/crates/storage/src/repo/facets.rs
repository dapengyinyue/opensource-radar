//! 筛选 facet 与源状态查询。

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgExecutor};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Facet {
    pub key: String,
    pub count: i64,
}

pub async fn languages<'e, E>(exec: E, limit: i64) -> Result<Vec<Facet>>
where
    E: PgExecutor<'e>,
{
    let rows = sqlx::query_as::<_, Facet>(
        "SELECT language AS key, count(*) AS count \
         FROM projects WHERE language IS NOT NULL \
         GROUP BY language ORDER BY count DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(exec)
    .await?;
    Ok(rows)
}

pub async fn topics<'e, E>(exec: E, limit: i64) -> Result<Vec<Facet>>
where
    E: PgExecutor<'e>,
{
    let rows = sqlx::query_as::<_, Facet>(
        "SELECT topic AS key, count(*) AS count \
         FROM (SELECT unnest(topics) AS topic FROM projects) t \
         GROUP BY topic ORDER BY count DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(exec)
    .await?;
    Ok(rows)
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SourceStatus {
    pub source: String,
    pub last_collected_at: Option<DateTime<Utc>>,
    pub project_count: i64,
}

pub async fn sources_status<'e, E>(exec: E) -> Result<Vec<SourceStatus>>
where
    E: PgExecutor<'e>,
{
    let rows = sqlx::query_as::<_, SourceStatus>(
        "SELECT 'github'::text AS source, MAX(last_collected_at) AS last_collected_at, \
                count(*) AS project_count \
         FROM projects WHERE 'github'::source_kind = ANY(source_kinds) \
         UNION ALL \
         SELECT 'hackernews'::text, MAX(last_collected_at), count(*) \
         FROM projects WHERE 'hackernews'::source_kind = ANY(source_kinds)",
    )
    .fetch_all(exec)
    .await?;
    Ok(rows)
}
