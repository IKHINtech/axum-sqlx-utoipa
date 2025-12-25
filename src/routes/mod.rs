use axum::Router;

use crate::db::DbPool;

pub mod admin;
pub mod auth;
pub mod cart;
pub mod doc;
pub mod health;
pub mod orders;
pub mod products;

// Build the API router without binding state; it will be provided at the top level.
pub fn create_api_router() -> Router<DbPool> {
    Router::new()
        .nest("/products", products::router())
        .nest("/auth", auth::router())
        .nest("/cart", cart::router())
        .nest("/orders", orders::route())
        .nest("/admin", admin::router())
}
