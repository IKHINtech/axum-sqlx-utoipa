use axum::{
    Json, Router,
    extract::State,
    routing::{delete, get},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::DbPool,
    error::AppResult,
    middleware::auth::AuthUser,
    models::CartItem,
    response::{ApiResponse, Meta},
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddToCartRequest {
    pub product_id: Uuid,
    pub quantity: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CartList {
    pub items: Vec<CartItem>,
}

pub fn router() -> Router<DbPool> {
    Router::new()
        .route("/", get(cart_list).post(add_to_cart))
        .route("/{product_id}", delete(remove_from_cart))
}

pub async fn cart_list(
    State(pool): State<DbPool>,
    user: AuthUser,
) -> AppResult<Json<ApiResponse<CartList>>> {
    let items = sqlx::query_as::<_, CartItem>("SELECT * FROM cart_items where user_id = $1")
        .bind(user.user_id)
        .fetch_all(&pool)
        .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cart_items WHERE user_id = $1")
        .bind(user.user_id)
        .fetch_one(&pool)
        .await?;

    let meta = Meta::new(1, total.0, total.0);

    let data = CartList { items };

    Ok(Json(ApiResponse::success("OK", data, Some(meta))))
}

pub async fn add_to_cart(
    State(pool): State<DbPool>,
    user: AuthUser,
    Json(payload): Json<AddToCartRequest>,
) -> String {
    "add to cart".to_string()
}

pub async fn remove_from_cart() -> String {
    "remove from cart".to_string()
}
