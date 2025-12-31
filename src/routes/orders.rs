use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use uuid::Uuid;

use crate::{
    dto::orders::{CheckoutRequest, OrderList, OrderWithItems, PayOrderRequest},
    error::AppResult,
    middleware::auth::AuthUser,
    response::ApiResponse,
    routes::params::OrderListQuery,
    services::order_service,
    state::AppState,
};

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/", get(list_order))
        .route("/checkout", post(checkout))
        .route("/{id}/pay", post(pay_order))
        .route("/{id}", get(get_order))
}

#[utoipa::path(
    get,
    path = "/api/orders",
    params(
        ("page" = Option<i64>, Query, description = "Page number, default 1"),
        ("per_page" = Option<i64>, Query, description = "Items per page, default 20"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("sort_order" = Option<String>, Query, description = "Sort order: asc, desc")
    ),
    responses(
        (status = 200, description = "List orders for current user", body = ApiResponse<OrderList>)
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn list_order(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<OrderListQuery>,
) -> AppResult<Json<ApiResponse<OrderList>>> {
    let resp = order_service::list_orders(&state, &user, query).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/orders/checkout",
    request_body = CheckoutRequest,
    responses(
        (status = 200, description = "Checkout current cart into an order", body = ApiResponse<OrderWithItems>),
        (status = 400, description = "Cart empty or validation error"),
    )
    , security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn checkout(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CheckoutRequest>,
) -> AppResult<Json<ApiResponse<OrderWithItems>>> {
    let resp = order_service::checkout(&state, &user, payload).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/orders/{id}/pay",
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    request_body = PayOrderRequest,
    responses(
        (status = 200, description = "Mark order as paid", body = ApiResponse<OrderWithItems>),
        (status = 400, description = "Invalid order state"),
        (status = 404, description = "Order not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn pay_order(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<PayOrderRequest>,
) -> AppResult<Json<ApiResponse<OrderWithItems>>> {
    let resp = order_service::pay_order(&state, &user, id, payload).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/orders/{id}",
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Get order with items", body = ApiResponse<OrderWithItems>),
        (status = 404, description = "Order not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn get_order(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<OrderWithItems>>> {
    let resp = order_service::get_order(&state, &user, id).await?;
    Ok(Json(resp))
}
