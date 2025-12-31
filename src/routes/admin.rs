use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, patch},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::AppResult,
    middleware::auth::AuthUser,
    models::{Order, Product},
    response::ApiResponse,
    routes::params::{OrderListQuery, Pagination },
    dto::orders::{OrderList, OrderWithItems},
    services::admin_service,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list_all_orders))
        .route("/orders/{id}", get(get_order_admin))
        .route("/orders/{id}/status", patch(update_order_status))
        .route("/inventory/low-stock", get(list_low_stock))
        .route("/inventory/{id}", patch(adjust_inventory))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrderStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LowStockQuery {
    #[serde(flatten)]
    pub pagination: Pagination,
    pub threshold: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InventoryAdjustRequest {
    pub delta: i32,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ProductList {
    pub items: Vec<Product>,
}

#[utoipa::path(
    get,
    path = "/admin/orders",
    params(
        ("page" = Option<i64>, Query, description = "Page number, default 1"),
        ("per_page" = Option<i64>, Query, description = "Items per page, default 20"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("sort_order" = Option<String>, Query, description = "Sort order: asc, desc")
    ),
    responses(
    (status = 200, description = "Get all orders (admin only)", body = ApiResponse<OrderList>),
    (status = 403, description = "Forbidden"),
    (status = 500, description = "Internal Server Error"),
    ),
    security(("bearer_auth" = [])),
    tag = "Admin"
)]
pub async fn list_all_orders(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<OrderListQuery>,
) -> AppResult<Json<ApiResponse<OrderList>>> {
    let resp = admin_service::list_all_orders(&state, &user, query).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/admin/orders/{id}",
    params(
    (
        "id" = Uuid, Path, description = "Order ID")
    ),
    responses(
    (status = 200, description = "Get any order with items (admin only)", body = ApiResponse<OrderWithItems>),
    (status = 404, description = "Not Found", ),
    (status = 403, description = "Forbidden", ),
    ),
    security(("bearer_auth" = [])),
    tag = "Admin"

)]
pub async fn get_order_admin(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<OrderWithItems>>> {
    let resp = admin_service::get_order_admin(&state, &user, id).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    patch,
    path = "/admin/orders/{id}/status",
    params(
    (
        "id" = Uuid, Path, description = "Order ID")
    ),
    request_body = UpdateOrderStatusRequest,
    responses(
        (status = 200, description = "Update order status", body = ApiResponse<Order>),
        (status = 400, description = "Invalid status"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not Found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Admin"
)]
pub async fn update_order_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateOrderStatusRequest>,
) -> AppResult<Json<ApiResponse<Order>>> {
    let resp = admin_service::update_order_status(&state, &user, id, payload).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/admin/inventory/low-stock",
    params(
        ("threshold" = Option<i32>, Query, description = "Stock threshold, default 5"),
        ("page" = Option<i64>, Query, description = "Page number, default 1"),
        ("per_page" = Option<i64>, Query, description = "Items per page, default 20")
    ),
    responses(
        (status = 200, description = "List low stock products", body = ApiResponse<ProductList>),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Admin"
)]
pub async fn list_low_stock(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<LowStockQuery>,
) -> AppResult<Json<ApiResponse<ProductList>>> {
    let resp = admin_service::list_low_stock(&state, &user, query).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    patch,
    path = "/admin/inventory/{id}",
    params(
    (
        "id" = Uuid, Path, description = "Product ID")
    ),
    request_body = InventoryAdjustRequest,
    responses(
        (status = 200, description = "Adjust inventory", body = ApiResponse<Product>),
        (status = 400, description = "Invalid adjustment"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not Found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Admin"
)]
pub async fn adjust_inventory(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<InventoryAdjustRequest>,
) -> AppResult<Json<ApiResponse<Product>>> {
    let resp = admin_service::adjust_inventory(&state, &user, id, payload).await?;
    Ok(Json(resp))
}
