use argon2::{
    Argon2, PasswordHasher,
    password_hash::{PasswordHash, PasswordVerifier, SaltString},
};
use axum::{Json, Router, extract::State, routing::post};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use password_hash::rand_core::OsRng;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::DbPool,
    error::{AppError, AppResult},
    models::User,
    response::{ApiResponse, Meta},
};

#[derive(Deserialize, Debug, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub fn router() -> Router<DbPool> {
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
    tag = "auth"
)]
pub async fn register(
    State(pool): State<DbPool>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<ApiResponse<User>>> {
    let RegisterRequest { email, password } = payload;
    let exist: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE email = $1")
        .bind(email.as_str())
        .fetch_optional(&pool)
        .await?;

    if exist.is_some() {
        return Err(AppError::BadRequest("Email is already taken".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?
        .to_string();

    let id = Uuid::new_v4();

    let user = sqlx::query_as(
        "INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(id)
    .bind(email.as_str())
    .bind(password_hash)
    .fetch_one(&pool)
    .await?;
    Ok(Json(ApiResponse::success("User created", user, None)))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login user", body = ApiResponse<LoginResponse>),
        (status = 400, description = "Invalid credentials")
    ),
    tag = "auth"
)]
pub async fn login(
    State(pool): State<DbPool>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    let LoginRequest { email, password } = payload;
    let user: Option<User> = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email.as_str())
        .fetch_optional(&pool)
        .await?;

    let user = match user {
        Some(u) => u,
        None => return Err(AppError::BadRequest("Invalid email or password".into())),
    };

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid password hash")))?;

    let argon2 = Argon2::default();
    if argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(AppError::BadRequest("Invalid email or password".into()));
    }

    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("JWT_SECRET is not set")))?;

    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to set expiration")))?;

    let claims = Claims {
        sub: user.id.to_string(),
        exp: expiration.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    let resp = LoginResponse {
        token: format!("Bearer {}", token),
    };

    Ok(Json(ApiResponse::success(
        "Logged in",
        resp,
        Some(Meta::empty()),
    )))
}
