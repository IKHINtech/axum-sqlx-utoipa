use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

use crate::response::{ApiResponse, Meta};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not Found")]
    NotFound,

    #[error("Bad Request {0}")]
    BadRequest(String),

    #[error("Forbidden")]
    Forbidden,

    #[error("Database error")]
    DbError(#[from] sqlx::Error),

    #[error("ORM error")]
    OrmError(#[from] sea_orm::DbErr),

    #[error("Internal Server Error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorData {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::DbError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::OrmError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = ApiResponse {
            message,
            data: Some(ErrorData {
                error: self.to_string(),
            }),
            meta: Some(Meta::empty()),
        };

        (status, axum::Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
