use axum::Json;
use serde::Serialize;

use crate::response::{ApiResponse, Meta};

#[derive(Serialize)]
pub struct HealthData {
    status: String,
}

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
