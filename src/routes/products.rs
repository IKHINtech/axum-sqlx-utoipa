use axum::{
    Json, Router,
    extract::{Path, Query, State}, routing::{delete, get, post, put},
};
use uuid::Uuid;

use crate::{
    dto::products::{CreateProductRequest, ProductList, UpdateProductRequest},
    error::AppResult,
    middleware::auth::AuthUser,
    models::Product,
    response::ApiResponse,
    routes::params::ProductQuery,
    services::product_service,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_product))
        .route("/", get(list_products))
        .route("/{id}", get(get_product))
        .route("/{id}", put(update_product))
        .route("/{id}", delete(delete_product))
}

#[utoipa::path(
    get,
    path = "/api/products",
    params(
        ("page" = Option<i64>, Query, description = "Page number, default 1"),
        ("per_page" = Option<i64>, Query, description = "Items per page, default 20"),
        ("q" = Option<String>, Query, description = "Search term for name/description"),
        ("min_price" = Option<i64>, Query, description = "Minimum price"),
        ("max_price" = Option<i64>, Query, description = "Maximum price"),
        ("sort_by" = Option<String>, Query, description = "Sort by: created_at, price, name"),
        ("sort_order" = Option<String>, Query, description = "Sort order: asc, desc")
    ),
    responses(
        (status = 200, description = "List products", body = ApiResponse<ProductList>)
    ),
    tag = "Products"
)]
pub async fn list_products(
    State(state): State<AppState>,
    Query(query): Query<ProductQuery>,
) -> AppResult<Json<ApiResponse<ProductList>>> {
    let resp = product_service::list_products(&state, query).await?;
    Ok(Json(resp))
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
    tag = "Products"
)]

pub async fn get_product(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Product>>> {
    let resp = product_service::get_product(&state, id).await?;
    Ok(Json(resp))
}
#[utoipa::path(
    post,
    path = "/api/products",
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Create product", body = ApiResponse<Product>)
    ),
    security(("bearer_auth" = [])),
    tag = "Products"
)]

pub async fn create_product(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateProductRequest>,
) -> AppResult<Json<ApiResponse<Product>>> {
    let resp = product_service::create_product(&state, &user, payload).await?;
    Ok(Json(resp))
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
    security(("bearer_auth" = [])),
    tag = "Products"
)]

pub async fn update_product(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProductRequest>,
) -> AppResult<Json<ApiResponse<Product>>> {
    let resp = product_service::update_product(&state, &user, id, payload).await?;
    Ok(Json(resp))
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
    security(("bearer_auth" = [])),
    tag = "Products"
)]

pub async fn delete_product(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let resp = product_service::delete_product(&state, &user, id).await?;
    Ok(Json(resp))
}
