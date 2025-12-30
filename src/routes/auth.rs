use axum::{Json, Router, extract::State, routing::post};

use crate::{
    dto::auth::{LoginRequest, LoginResponse, RegisterRequest},
    error::AppResult,
    models::User,
    response::ApiResponse,
    services::auth_service::{login_user, register_user},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Register user", body = ApiResponse<User>)
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<ApiResponse<User>>> {
    let resp = register_user(&state.pool, payload).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login user", body = ApiResponse<LoginResponse>),
        (status = 400, description = "Invalid credentials")
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    let resp = login_user(&state.pool, payload).await?;
    Ok(Json(resp))
}
