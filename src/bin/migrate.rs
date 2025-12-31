use axum_ecommerce_api::{config::AppConfig, db::{create_orm_conn, run_migrations}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let orm = create_orm_conn(&config.database_url).await?;
    run_migrations(&orm).await?;
    println!("Migrations applied");
    Ok(())
}
