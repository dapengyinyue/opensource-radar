use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// 建立连接池。migrations 目录相对本 crate（backend/crates/storage）为 `../../migrations`。
pub async fn create_pool(url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(url)
        .await?;
    Ok(pool)
}

/// 运行嵌入的迁移脚本。
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}

/// 数据库存活探针。
pub async fn ping(pool: &PgPool) -> Result<()> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}
