//! repository：projects / snapshots / raw 的读写。

pub mod facets;
pub mod project;
pub mod raw;
pub mod snapshot;

use anyhow::Result;
use domain::normalize;
use domain::source::RawItem;
use sqlx::PgPool;

/// 编排一次原始项的完整落库：归一 → upsert project → 写 snapshot → upsert raw。
/// 在单事务内执行。返回 project id。
pub async fn persist_raw_item(pool: &PgPool, item: &RawItem) -> Result<i64> {
    let rec = normalize::normalize(item);
    let mut tx = pool.begin().await?;

    let project_id = project::upsert_by_key(&mut *tx, &rec).await?;
    snapshot::write_snapshot(&mut *tx, project_id, rec.stars, rec.hn_points).await?;

    match item {
        RawItem::GithubRepo(g) => raw::upsert_github(&mut *tx, project_id, g).await?,
        RawItem::HnStory(h) => raw::upsert_hn(&mut *tx, project_id, h).await?,
    }

    tx.commit().await?;
    Ok(project_id)
}
