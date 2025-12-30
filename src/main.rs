use axum::{
    Json, Router,
    http::{HeaderName, Request, Response, StatusCode, Uri},
    routing::get,
};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::{net::SocketAddr, time::Duration};

use axum_ecommerce_api::{
    config::AppConfig,
    db::create_pool,
    response::{ApiResponse, Meta},
    routes::{create_api_router, doc::scalar_docs, health},
};

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
    let concurrency_limit_layer = ConcurrencyLimitLayer::new(100);

    let request_id_header = HeaderName::from_static("x-request-id");
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
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
        })
        .on_request(|request: &Request<_>, _span: &tracing::Span| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("-");
            tracing::info!(
                request_id = %request_id,
                method = %request.method(),
                uri = %request.uri(),
                "request started"
            );
        })
        .on_response(|response: &Response<_>, latency: Duration, _span: &tracing::Span| {
            tracing::info!(
                status = %response.status(),
                ms = %latency.as_millis(),
                "request finished"
            );
        });

    let app = Router::new()
        .route("/health", get(health::health_check))
        .nest("/api", api_router)
        .merge(scalar_docs())
        .fallback(not_found)
        .layer(trace_layer)
        .layer(PropagateRequestIdLayer::new(
            request_id_header.clone(),
        ))
        .layer(SetRequestIdLayer::new(
            request_id_header,
            MakeRequestUuid,
        ))
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(concurrency_limit_layer)
        .with_state(pool);

    let addr = SocketAddr::from((config.host.parse::<std::net::IpAddr>()?, config.port));
    tracing::info!("listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}

async fn not_found(uri: Uri) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let body = ApiResponse::success(
        "Not Found",
        serde_json::json!({ "path": uri.path() }),
        Some(Meta::empty()),
    );
    (StatusCode::NOT_FOUND, Json(body))
}
