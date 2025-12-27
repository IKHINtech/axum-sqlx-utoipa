use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::DbPool,
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Favorite, Product},
    response::{ApiResponse, Meta},
};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AddFavoriteRequest {
    pub product_id: Uuid,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct FavoriteProductList {
    pub items: Vec<Product>,
}

pub fn router() -> Router<DbPool> {
    Router::new()
        .route("/", get(list_favorites).post(add_favorite))
        .route("/{product_id}", delete(remove_favorite))
}

#[utoipa::path(
    post,
    path = "/favorites",
    tag = "favorites",
    operation_id = "add_favorite",
    request_body = AddFavoriteRequest,
    responses(
        (status = 200, description = "OK", body = ApiResponse<Favorite>),
        (status = 400, description = "Bad Request", body = ApiResponse<serde_json::Value>),
        (status = 401, description = "Unauthorized", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Not Found", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn remove_favorite(
    State(pool): State<DbPool>,
    user: AuthUser,
    Path(product_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = sqlx::query("DELETE FROM favorites WHERE user_id = $1 AND product_id = $2")
        .bind(user.user_id)
        .bind(product_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(ApiResponse::success(
        "Removed from favorites",
        serde_json::json!({}),
        Some(Meta::empty()),
    )))
}

#[utoipa::path(
    get,
    path = "/favorites",
    tag = "favorites",
    operation_id = "list_favorites",
    responses(
        (status = 200, description = "OK", body = ApiResponse<FavoriteProductList>),
        (status = 401, description = "Unauthorized", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Not Found", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn list_favorites(
    State(db): State<DbPool>,
    user: AuthUser,
) -> AppResult<Json<ApiResponse<FavoriteProductList>>> {
    let products = sqlx::query_as::<_, Product>(
        r#"
        SELECT p.*
        FROM favorites f
        JOIN products p ON p.id = f.product_id
        WHERE f.user_id = $1
        ORDER BY f.created_at DESC
        "#,
    )
    .bind(user.user_id)
    .fetch_all(&db)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM favorites WHERE user_id = $1")
        .bind(user.user_id)
        .fetch_one(&db)
        .await?;

    let meta = Meta::new(1, total.0, total.0);

    let data = FavoriteProductList { items: products };

    Ok(Json(ApiResponse::success("OK", data, Some(meta))))
}

#[utoipa::path(
    post,
    path = "/favorites/{product_id}",
    tag = "favorites",
    operation_id = "add_favorite",
    request_body = AddFavoriteRequest,
    responses(
        (status = 200, description = "OK", body = ApiResponse<Favorite>),
        (status = 400, description = "Bad Request", body = ApiResponse<serde_json::Value>),
        (status = 401, description = "Unauthorized", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Not Found", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn add_favorite(
    State(pool): State<DbPool>,
    user: AuthUser,
    Json(payload): Json<AddFavoriteRequest>,
) -> AppResult<Json<ApiResponse<Favorite>>> {
    // cek apakah product ada
    let product_exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM products WHERE id = $1")
        .bind(payload.product_id)
        .fetch_optional(&pool)
        .await?;

    if product_exists.is_none() {
        return Err(AppError::BadRequest("Product not found".into()));
    }

    // cek apakah favorite sudah ada
    let existing: Option<Favorite> =
        sqlx::query_as("SELECT * FROM favorites WHERE user_id = $1 AND product_id = $2")
            .bind(user.user_id)
            .bind(payload.product_id)
            .fetch_optional(&pool)
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
        .fetch_one(&pool)
        .await?
    };

    Ok(Json(ApiResponse::success(
        "Added to favorites",
        favorite,
        Some(Meta::empty()),
    )))
}
