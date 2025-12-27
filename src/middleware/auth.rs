use axum::{extract::FromRequestParts, http::header};
use jsonwebtoken::{DecodingKey, Validation, decode};
use uuid::Uuid;

use crate::{error::AppError, routes::auth::Claims};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: String,
}

pub fn ensure_role(user: &AuthUser, role: &str) -> Result<(), AppError> {
    if user.role != role {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub fn ensure_admin(user: &AuthUser) -> Result<(), AppError> {
    ensure_role(user, "admin")
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .ok_or_else(|| AppError::BadRequest("Missing Authorization header".into()))?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| AppError::BadRequest("Invalid Authorization header".into()))?;

        if !auth_str.starts_with("Bearer ") {
            return Err(AppError::BadRequest("Invalid Authorization scheme".into()));
        }
        let token = auth_str.trim_start_matches("Bearer ").trim();

        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| AppError::Internal(anyhow::anyhow!("JWT_SECRET is not set")))?;

        let decoded = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::BadRequest("Invalid or expired token".into()))?;

        let user_id = Uuid::parse_str(&decoded.claims.sub)
            .map_err(|_| AppError::BadRequest("Invalid user id in token".into()))?;

        Ok(AuthUser {
            user_id,
            role: decoded.claims.role.clone(),
        })
    }
}
