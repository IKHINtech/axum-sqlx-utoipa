use axum::Router;

use crate::db::DbPool;

pub mod auth;
pub mod doc;
pub mod health;
pub mod products;

// Build the API router without binding state; it will be provided at the top level.
pub fn create_api_router() -> Router<DbPool> {
    Router::new()
        .nest("/products", products::router())
        .nest("/auth", auth::router())
}
