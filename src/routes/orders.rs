use axum::{
    Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    db::DbPool,
    models::{Order, Product},
};

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct OrderList {
    pub items: Vec<Order>,
}

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct OrderWithItems {
    pub order: Order,
    pub products: Vec<Product>,
}

pub fn route() -> Router<DbPool> {
    Router::new()
        .route("/", get(list_order))
        .route("/checkout", post(checkout))
        .route("/{id}", get(get_order))
}

#[utoipa::path(get, path = "/orders", tag = "Orders")]
pub async fn list_order() -> String {
    "list".to_string()
}
#[utoipa::path(post, path = "/orders/checkout", tag = "Orders")]
pub async fn checkout() -> String {
    "checkout".to_string()
}

#[utoipa::path(get, path = "/orders/{id}", tag = "Orders")]
pub async fn get_order() -> String {
    "get".to_string()
}
