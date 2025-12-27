use axum::{
    Json, Router,
    http::{Request, StatusCode},
    routing::get,
};
use tower_governor::errors::GovernorError;
use tower_governor::{GovernorConfigBuilder, GovernorLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::net::SocketAddr;

use crate::{
    config::AppConfig,
    db::create_pool,
    response::{ApiResponse, Meta},
    routes::{create_api_router, doc::scalar_docs},
};

mod audit;
mod config;
mod db;
mod error;
mod middleware;
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
    let governor_config = GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(30)
        .finish()
        .expect("failed to build rate limit config");
    let rate_limit_layer = GovernorLayer::new(governor_config);
    let trace_layer = TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("-");
        tracing::info_span!(
            "http_request",
            method = %request.method(),
            uri = %request.uri(),
            request_id = %request_id
        )
    });

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .nest("/api", api_router)
        .merge(scalar_docs())
        .layer(trace_layer)
        .layer(PropagateRequestIdLayer::new())
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(rate_limit_layer)
        .layer(axum::error_handling::HandleErrorLayer::new(
            |err: GovernorError| async move {
                let body = ApiResponse::success(
                    "Too Many Requests",
                    serde_json::json!({ "error": err.to_string() }),
                    Some(Meta::empty()),
                );
                (StatusCode::TOO_MANY_REQUESTS, Json(body))
            },
        ))
        .layer(SetRequestIdLayer::new(MakeRequestUuid))
        .with_state(pool);

    let addr = SocketAddr::from((config.host.parse::<std::net::IpAddr>()?, config.port));
    tracing::info!("listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
