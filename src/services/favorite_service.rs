use uuid::Uuid;

use crate::{
    audit::log_audit,
    db::DbPool,
    dto::favorites::{AddFavoriteRequest, FavoriteProductList},
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Favorite, Product},
    response::{ApiResponse, Meta},
    routes::params::Pagination,
};

pub async fn list_favorites(
    db: &DbPool,
    user: &AuthUser,
    pagination: Pagination,
) -> AppResult<ApiResponse<FavoriteProductList>> {
    let (page, limit, offset) = pagination.normalize();
    let products = sqlx::query_as::<_, Product>(
        r#"
        SELECT p.*
        FROM favorites f
        JOIN products p ON p.id = f.product_id
        WHERE f.user_id = $1
        ORDER BY f.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM favorites WHERE user_id = $1")
        .bind(user.user_id)
        .fetch_one(db)
        .await?;

    let meta = Meta::new(page, limit, total.0);
    let data = FavoriteProductList { items: products };
    Ok(ApiResponse::success("OK", data, Some(meta)))
}

pub async fn add_favorite(
    pool: &DbPool,
    user: &AuthUser,
    payload: AddFavoriteRequest,
) -> AppResult<ApiResponse<Favorite>> {
    let product_exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM products WHERE id = $1")
        .bind(payload.product_id)
        .fetch_optional(pool)
        .await?;

    if product_exists.is_none() {
        return Err(AppError::BadRequest("Product not found".into()));
    }

    let existing: Option<Favorite> =
        sqlx::query_as("SELECT * FROM favorites WHERE user_id = $1 AND product_id = $2")
            .bind(user.user_id)
            .bind(payload.product_id)
            .fetch_optional(pool)
            .await?;

    let favorite = if let Some(fav) = existing {
        fav
    } else {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Favorite>(
            r#"
            INSERT INTO favorites (id, user_id, product_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user.user_id)
        .bind(payload.product_id)
        .fetch_one(pool)
        .await?
    };

    if let Err(err) = log_audit(
        pool,
        Some(user.user_id),
        "favorite_add",
        Some("favorites"),
        Some(serde_json::json!({ "product_id": payload.product_id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Added to favorites",
        favorite,
        Some(Meta::empty()),
    ))
}

pub async fn remove_favorite(
    pool: &DbPool,
    user: &AuthUser,
    product_id: Uuid,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM favorites WHERE user_id = $1 AND product_id = $2")
        .bind(user.user_id)
        .bind(product_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    if let Err(err) = log_audit(
        pool,
        Some(user.user_id),
        "favorite_remove",
        Some("favorites"),
        Some(serde_json::json!({ "product_id": product_id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Removed from favorites",
        serde_json::json!({}),
        Some(Meta::empty()),
    ))
}
