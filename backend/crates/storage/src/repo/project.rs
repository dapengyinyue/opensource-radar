use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor};

use domain::models::NormalizedRecord;

/// projects 行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
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
    pub github_created_at: Option<DateTime<Utc>>,
    pub github_updated_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub source_kinds: Vec<String>,
    pub metadata: serde_json::Value,
    pub first_seen_at: DateTime<Utc>,
    pub last_collected_at: DateTime<Utc>,
}

const UPSERT_SQL: &str = r#"
INSERT INTO projects (
  dedup_key, name, full_name, description, repo_url, homepage_url, language,
  topics, stars, forks, open_issues, hn_points, hn_comment_count,
  github_created_at, github_updated_at, last_activity_at, source_kinds, metadata,
  first_seen_at, last_collected_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
        ARRAY[$17::source_kind], $18, $19, $20)
ON CONFLICT (dedup_key) DO UPDATE SET
  name              = CASE WHEN $17 = 'github' THEN EXCLUDED.name ELSE projects.name END,
  full_name         = CASE WHEN $17 = 'github' THEN EXCLUDED.full_name ELSE projects.full_name END,
  description       = CASE WHEN $17 = 'github' THEN EXCLUDED.description ELSE projects.description END,
  topics            = CASE WHEN $17 = 'github' THEN EXCLUDED.topics ELSE projects.topics END,
  repo_url          = COALESCE(EXCLUDED.repo_url, projects.repo_url),
  homepage_url      = COALESCE(EXCLUDED.homepage_url, projects.homepage_url),
  language          = COALESCE(EXCLUDED.language, projects.language),
  stars             = COALESCE(EXCLUDED.stars, projects.stars),
  forks             = COALESCE(EXCLUDED.forks, projects.forks),
  open_issues       = COALESCE(EXCLUDED.open_issues, projects.open_issues),
  hn_points         = COALESCE(EXCLUDED.hn_points, projects.hn_points),
  hn_comment_count  = COALESCE(EXCLUDED.hn_comment_count, projects.hn_comment_count),
  github_created_at = COALESCE(EXCLUDED.github_created_at, projects.github_created_at),
  github_updated_at = COALESCE(EXCLUDED.github_updated_at, projects.github_updated_at),
  last_activity_at  = GREATEST(EXCLUDED.last_activity_at, projects.last_activity_at),
  source_kinds      = (SELECT array_agg(DISTINCT x) FROM unnest(array_cat(projects.source_kinds, ARRAY[$17::source_kind])) AS t(x)),
  metadata          = EXCLUDED.metadata,
  last_collected_at = EXCLUDED.last_collected_at,
  updated_at        = now()
RETURNING id
"#;

/// 以 dedup_key upsert 一个归一记录，返回 project id。
/// text 类权威字段仅 GitHub 源覆盖；metric 类用 COALESCE 取非空；source_kinds 取并集。
pub async fn upsert_by_key<'e, E>(exec: E, rec: &NormalizedRecord) -> Result<i64>
where
    E: PgExecutor<'e>,
{
    let now = Utc::now();
    let sk = rec.source_kind.as_str();
    let id: i64 = sqlx::query_scalar(UPSERT_SQL)
        .bind(&rec.dedup_key)
        .bind(&rec.name)
        .bind(&rec.full_name)
        .bind(&rec.description)
        .bind(&rec.repo_url)
        .bind(&rec.homepage_url)
        .bind(&rec.language)
        .bind(&rec.topics)
        .bind(rec.stars)
        .bind(rec.forks)
        .bind(rec.open_issues)
        .bind(rec.hn_points)
        .bind(rec.hn_comment_count)
        .bind(rec.github_created_at)
        .bind(rec.github_updated_at)
        .bind(rec.last_activity_at)
        .bind(sk)
        .bind(&rec.metadata)
        .bind(now)
        .bind(now)
        .fetch_one(exec)
        .await?;
    Ok(id)
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Sort {
    #[default]
    Hottest,
    Stars,
    Recent,
    HnPoints,
}

impl Sort {
    pub fn parse(s: &str) -> Self {
        match s {
            "stars" => Sort::Stars,
            "recent" => Sort::Recent,
            "hn_points" => Sort::HnPoints,
            _ => Sort::Hottest,
        }
    }

    fn order_by(&self) -> &'static str {
        match self {
            Sort::Hottest => "(COALESCE(stars,0)/50.0 + COALESCE(hn_points,0)/5.0) \
                * GREATEST(0, 1 - EXTRACT(epoch FROM (now() - COALESCE(last_activity_at, now())))/86400.0/180.0) DESC NULLS LAST",
            Sort::Stars => "stars DESC NULLS LAST",
            Sort::Recent => "last_activity_at DESC NULLS LAST",
            Sort::HnPoints => "hn_points DESC NULLS LAST",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProjectFilter {
    pub language: Option<String>,
    pub topic: Option<String>,
    /// Some("github"|"hackernews") 或 None=all
    pub source: Option<String>,
    /// 全文搜索：匹配 name / full_name / description（ILIKE）
    pub q: Option<String>,
    pub sort: Sort,
    /// 活跃时间下限（last_activity_at >= since）
    pub since: Option<DateTime<Utc>>,
    /// 首次发现时间下限（first_seen_at >= first_seen_since），日报「今日新发现」用
    pub first_seen_since: Option<DateTime<Utc>>,
    pub page: i64,
    pub per_page: i64,
}

const SELECT_COLS: &str = r#"
SELECT id, dedup_key, name, full_name, description, repo_url, homepage_url, language,
       topics, stars, forks, open_issues, hn_points, hn_comment_count,
       github_created_at, github_updated_at, last_activity_at,
       source_kinds::text[] AS source_kinds,
       metadata, first_seen_at, last_collected_at
FROM projects
"#;

/// 榜单分页查询。
pub async fn list<'e, E>(exec: E, f: &ProjectFilter) -> Result<Vec<Project>>
where
    E: PgExecutor<'e>,
{
    let per_page = f.per_page.clamp(1, 100);
    let offset = (f.page.max(1) - 1) * per_page;
    let sql = format!(
        "{SELECT_COLS} \
         WHERE ($1::text IS NULL OR language = $1) \
           AND ($2::text IS NULL OR $2 = ANY(topics)) \
           AND ($3::text IS NULL OR $3::source_kind = ANY(source_kinds)) \
           AND ($4::timestamptz IS NULL OR last_activity_at >= $4) \
           AND ($5::text IS NULL OR name ILIKE '%' || $5 || '%' \
                OR full_name ILIKE '%' || $5 || '%' \
                OR description ILIKE '%' || $5 || '%') \
           AND ($6::timestamptz IS NULL OR first_seen_at >= $6) \
         ORDER BY {} \
         LIMIT $7 OFFSET $8",
        f.sort.order_by()
    );
    let rows = sqlx::query_as::<_, Project>(&sql)
        .bind(&f.language)
        .bind(&f.topic)
        .bind(&f.source)
        .bind(f.since)
        .bind(&f.q)
        .bind(f.first_seen_since)
        .bind(per_page)
        .bind(offset)
        .fetch_all(exec)
        .await?;
    Ok(rows)
}

pub async fn get<'e, E>(exec: E, id: i64) -> Result<Option<Project>>
where
    E: PgExecutor<'e>,
{
    let sql = format!("{SELECT_COLS} WHERE id = $1");
    let row = sqlx::query_as::<_, Project>(&sql)
        .bind(id)
        .fetch_optional(exec)
        .await?;
    Ok(row)
}

/// 与 `list` 同过滤条件的计数。
pub async fn count<'e, E>(exec: E, f: &ProjectFilter) -> Result<i64>
where
    E: PgExecutor<'e>,
{
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projects \
         WHERE ($1::text IS NULL OR language = $1) \
           AND ($2::text IS NULL OR $2 = ANY(topics)) \
           AND ($3::text IS NULL OR $3::source_kind = ANY(source_kinds)) \
           AND ($4::timestamptz IS NULL OR last_activity_at >= $4) \
           AND ($5::text IS NULL OR name ILIKE '%' || $5 || '%' \
                OR full_name ILIKE '%' || $5 || '%' \
                OR description ILIKE '%' || $5 || '%') \
           AND ($6::timestamptz IS NULL OR first_seen_at >= $6)",
    )
    .bind(&f.language)
    .bind(&f.topic)
    .bind(&f.source)
    .bind(f.since)
    .bind(&f.q)
    .bind(f.first_seen_since)
    .fetch_one(exec)
    .await?;
    Ok(n)
}
