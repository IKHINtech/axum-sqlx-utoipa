use chrono::DateTime;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    audit::log_audit,
    db::DbPool,
    dto::cart::{AddToCartRequest, CartItemDto, CartList},
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::Product,
    response::{ApiResponse, Meta},
    routes::params::Pagination,
};

#[derive(FromRow)]
struct CartWithProductRow {
    cart_id: Uuid,
    quantity: i32,
    product_id: Uuid,
    name: String,
    description: Option<String>,
    price: i64,
    stock: i32,
    created_at: DateTime<chrono::Utc>,
}

pub async fn list_cart(
    pool: &DbPool,
    user: &AuthUser,
    pagination: Pagination,
) -> AppResult<ApiResponse<CartList>> {
    let (page, limit, offset) = pagination.normalize();
    let rows = sqlx::query_as::<_, CartWithProductRow>(
        r#"
        SELECT ci.id AS cart_id, ci.quantity,
               p.id AS product_id, p.name, p.description, p.price, p.stock, p.created_at
        FROM cart_items ci
        JOIN products p ON p.id = ci.product_id
        WHERE ci.user_id = $1
        ORDER BY ci.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM cart_items WHERE user_id = $1")
            .bind(user.user_id)
            .fetch_one(pool)
            .await?;

    let items = rows
        .into_iter()
        .map(|row| CartItemDto {
            id: row.cart_id,
            product: Product {
                id: row.product_id,
                name: row.name,
                description: row.description,
                price: row.price,
                stock: row.stock,
                created_at: row.created_at,
            },
            quantity: row.quantity,
        })
        .collect();

    let meta = Meta::new(page, limit, total.0);
    Ok(ApiResponse::success("OK", CartList { items }, Some(meta)))
}

pub async fn add_to_cart(
    pool: &DbPool,
    user: &AuthUser,
    payload: AddToCartRequest,
) -> AppResult<ApiResponse<crate::models::CartItem>> {
    if payload.quantity <= 0 {
        return Err(AppError::BadRequest(
            "quantity must be greater than 0".to_string(),
        ));
    }

    let product_exist: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM products WHERE id = $1")
        .bind(payload.product_id)
        .fetch_optional(pool)
        .await?;
    if product_exist.is_none() {
        return Err(AppError::BadRequest("product not found".to_string()));
    }

    let exist: Option<crate::models::CartItem> =
        sqlx::query_as("SELECT * FROM cart_items WHERE user_id = $1 AND product_id = $2")
            .bind(user.user_id)
            .bind(payload.product_id)
            .fetch_optional(pool)
            .await?;

    let cart_item = if let Some(item) = exist {
        sqlx::query_as::<_, crate::models::CartItem>(
            r#"
            UPDATE cart_items
            SET quantity = $3
            WHERE id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(item.id)
        .bind(user.user_id)
        .bind(payload.quantity)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_as("INSERT INTO cart_items (user_id, product_id, quantity) VALUES ($1, $2, $3) RETURNING *")
            .bind(user.user_id)
            .bind(payload.product_id)
            .bind(payload.quantity)
            .fetch_one(pool)
            .await?
    };

    if let Err(err) = log_audit(
        pool,
        Some(user.user_id),
        "cart_update",
        Some("cart_items"),
        Some(serde_json::json!({ "product_id": payload.product_id, "quantity": payload.quantity })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success("OK", cart_item, None))
}

pub async fn remove_from_cart(
    pool: &DbPool,
    user: &AuthUser,
    product_id: Uuid,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM cart_items WHERE product_id = $1 AND user_id = $2")
        .bind(product_id)
        .bind(user.user_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    if let Err(err) = log_audit(
        pool,
        Some(user.user_id),
        "cart_remove",
        Some("cart_items"),
        Some(serde_json::json!({ "product_id": product_id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Removed from cart",
        serde_json::json!({}),
        Some(Meta::empty()),
    ))
}
