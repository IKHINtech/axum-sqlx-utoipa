use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct Meta {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub total: Option<i64>,
}

impl Meta {
    pub fn new(page: i64, per_page: i64, total: i64) -> Self {
        Self {
            page: Some(page),
            per_page: Some(per_page),
            total: Some(total),
        }
    }

    pub fn empty() -> Self {
        Self {
            page: None,
            per_page: None,
            total: None,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    pub message: String,
    pub data: Option<T>,
    pub meta: Option<Meta>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(message: impl Into<String>, data: T, meta: Option<Meta>) -> Self {
        Self {
            message: message.into(),
            data: Some(data),
            meta,
        }
    }
}
