use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::net::SocketAddr;

use crate::{config::AppConfig, db::create_pool, routes::{create_api_router, doc::scalar_docs}};

mod config;
mod db;
mod error;
mod models;
mod response;
mod routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,axum_ecommerce_api=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;
    let pool = create_pool(&config.database_url).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let api_router = create_api_router();

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .nest("/api", api_router)
        .merge(scalar_docs())
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let addr = SocketAddr::from((config.host.parse::<std::net::IpAddr>()?, config.port));
    tracing::info!("listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
