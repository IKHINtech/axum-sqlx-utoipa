use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::response::{ApiResponse, Meta};

#[derive(Serialize, ToSchema)]
pub struct HealthData {
    status: String,
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "OK", body = ApiResponse<HealthData>),
    ),
        tag = "Health"
)]
pub async fn health_check() -> Json<ApiResponse<HealthData>> {
    let data = HealthData {
        status: "ok".to_string(),
    };

    Json(ApiResponse::success(
        "Health check",
        data,
        Some(Meta::empty()),
    ))
}
