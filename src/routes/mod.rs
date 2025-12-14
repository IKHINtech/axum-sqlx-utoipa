use axum::Router;

use crate::db::DbPool;

pub mod health;
pub mod products;
pub mod doc;

// Build the API router without binding state; it will be provided at the top level.
pub fn create_api_router() -> Router<DbPool> {
    Router::new().nest("/products", products::router())
}
