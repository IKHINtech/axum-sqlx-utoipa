use sqlx::{PgPool, postgres::PgPoolOptions};
use sea_orm::{Database, DatabaseConnection};

pub type DbPool = PgPool;
pub async fn create_pool(database_url: &str) -> anyhow::Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub type OrmConn = DatabaseConnection;

pub async fn create_orm_conn(database_url: &str) -> anyhow::Result<OrmConn> {
    let conn = Database::connect(database_url).await?;
    Ok(conn)
}
