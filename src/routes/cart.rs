use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::DbPool,
    error::{AppError, AppResult},
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

#[utoipa::path(
    get,
    path = "/api/cart",
    responses(
        (status = 200, description = "List cart items for current user", body = ApiResponse<CartList>)
    ),
    tag = "cart"
)]
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

#[utoipa::path(
    post,
    path = "/api/cart",
    request_body = AddToCartRequest,
    responses(
        (status = 200, description = "Add or update cart item", body = ApiResponse<CartItem>),
        (status = 400, description = "Bad request"),
    ),
    tag = "cart"
)]
pub async fn add_to_cart(
    State(pool): State<DbPool>,
    user: AuthUser,
    Json(payload): Json<AddToCartRequest>,
) -> AppResult<Json<ApiResponse<CartItem>>> {
    if payload.quantity <= 0 {
        return Err(AppError::BadRequest(
            "quantity must be greater than 0".to_string(),
        ));
    }
    let product_exist: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM products WHERE id = $1 ")
        .bind(payload.product_id)
        .fetch_optional(&pool)
        .await?;
    if product_exist.is_none() {
        return Err(AppError::BadRequest("product not found".to_string()));
    }
    let exist: Option<CartItem> =
        sqlx::query_as("SELECT * FROM cart_items WHERE user_id = $1 AND product_id = $2")
            .bind(payload.product_id)
            .fetch_optional(&pool)
            .await?;

    let cart_item = if let Some(item) = exist {
        sqlx::query_as::<_, CartItem>(
            r#"
            UPDATE cart_items
            SET quantity = $3
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(item.id)
        .bind(user.user_id)
        .bind(payload.quantity)
        .fetch_one(&pool)
        .await?
    } else {
        sqlx::query_as("INSERT INTO cart_items (user_id, product_id, quantity) VALUES ($1, $2, $3) RETURNING *")
            .bind(user.user_id)
            .bind(payload.product_id)
            .bind(payload.quantity)
            .fetch_one(&pool)
            .await?
    };
    Ok(Json(ApiResponse::success("OK", cart_item, None)))
}

#[utoipa::path(
    delete,
    path = "/api/cart/{product_id}",
    params(

        ("product_id" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "OK", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Cart item not found"),
    ),
    tag = "Cart"
)]
pub async fn remove_from_cart(
    State(pool): State<DbPool>,
    auht: AuthUser,
    Path(product_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = sqlx::query("DELETE from cart_items where product_id = $1 and user_id = $2")
        .bind(product_id)
        .bind(auht.user_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(ApiResponse::success(
        "Removed from cart",
        serde_json::json!({}),
        Some(Meta::empty()),
    )))
}
