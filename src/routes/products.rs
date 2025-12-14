use axum::{
    Json, Router,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::DbPool,
    error::{AppError, AppResult},
    models::Product,
    response::{ApiResponse, Meta},
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: String,
    pub price: i64,
    pub stock: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<i64>,
    pub stock: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct ProductList {
    pub items: Vec<Product>,
}

pub fn router() -> Router<DbPool> {
    Router::new()
        .route("/", axum::routing::post(create_product))
        .route("/", axum::routing::get(list_products))
        .route("/{id}", axum::routing::get(get_product))
        .route("/{id}", axum::routing::put(update_product))
        .route("/{id}", axum::routing::delete(delete_product))
}

#[utoipa::path(
    get,
    path = "/api/products",
    params(
        ("page" = Option<i64>, Query, description = "Page number, default 1"),
        ("per_page" = Option<i64>, Query, description = "Items per page, default 10"),
    ),
    responses(
        (status = 200, description = "List products", body = ApiResponse<ProductList>)
    ),
    tag = "products"
)]
pub async fn list_products(
    State(pool): State<DbPool>,
) -> AppResult<Json<ApiResponse<ProductList>>> {
    let page = 1_i64;
    let limit = 10_i64;
    let offset = (page - 1) * limit;
    let items = sqlx::query_as::<_, Product>(
        "SELECT * FROM products order by created_at LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT count(*) FROM products")
        .fetch_one(&pool)
        .await?;

    let meta = Meta::new(page, limit, total.0);
    let data = ProductList { items };
    Ok(Json(ApiResponse::success("Products", data, Some(meta))))
}
#[utoipa::path(
    get,
    path = "/api/products/{id}",
    params(
        ("id" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Get product", body = ApiResponse<Product>),
        (status = 404, description = "Product not found"),
    ),
    tag = "products"
)]

pub async fn get_product(
    Path(id): Path<Uuid>,
    State(pool): State<DbPool>,
) -> AppResult<Json<ApiResponse<Product>>> {
    let result = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?;
    let result = match result {
        Some(p) => p,
        None => return Err(AppError::NotFound),
    };
    Ok(Json(ApiResponse::success("Product", result, None)))
}
#[utoipa::path(
    post,
    path = "/api/products",
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Create product", body = ApiResponse<Product>)
    ),
    tag = "products"
)]

pub async fn create_product(
    State(pool): State<DbPool>,
    Json(payload): Json<CreateProductRequest>,
) -> AppResult<Json<ApiResponse<Product>>> {
    let id = Uuid::new_v4();
    let product = sqlx::query_as::<_, Product>(
        "INSERT INTO products (id, name, description, price, stock) VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(id)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.price)
    .bind(payload.stock)
    .fetch_one(&pool)
    .await?;

    Ok(Json(ApiResponse::success(
        "Product created",
        product,
        Some(Meta::empty()),
    )))
}
#[utoipa::path(
    put,
    path = "/api/products/{id}",
    params(
        ("id" = Uuid, Path, description = "Product ID")
    ),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "Updated product", body = ApiResponse<Product>)
    ),
    tag = "products"
)]

pub async fn update_product(
    State(pool): State<DbPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProductRequest>,
) -> AppResult<Json<ApiResponse<Product>>> {
    let existing = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?;
    let existing = match existing {
        Some(p) => p,
        None => return Err(AppError::NotFound),
    };

    let name = payload.name.unwrap_or(existing.name);
    let description = payload.description.or(existing.description);
    let price = payload.price.unwrap_or(existing.price);
    let stock = payload.stock.unwrap_or(existing.stock);

    let product = sqlx::query_as::<_, Product>(
        r#"
        UPDATE products
        SET name = $2, description = $3, price = $4, stock = $5
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(price)
    .bind(stock)
    .fetch_one(&pool)
    .await?;

    Ok(Json(ApiResponse::success(
        "Updated",
        product,
        Some(Meta::empty()),
    )))
}
#[utoipa::path(
    delete,
    path = "/api/products/{id}",
    params(
        ("id" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 204, description = "Deleted product")
    ),
    tag = "products"
)]

pub async fn delete_product(
    State(pool): State<DbPool>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(ApiResponse::success(
        "Deleted",
        serde_json::json!({}),
        Some(Meta::empty()),
    )))
}
