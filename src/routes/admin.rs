use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use uuid::Uuid;

use crate::{
    db::DbPool,
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Order, OrderItem},
    response::{ApiResponse, Meta},
    routes::orders::{OrderList, OrderWithItems},
};

#[derive(Debug, Clone)]
pub struct AdminGuard;

fn ensure_admin(user: &AuthUser) -> Result<(), AppError> {
    if user.role != "admin" {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub fn router() -> Router<DbPool> {
    Router::new()
        .route("/orders", get(list_all_orders))
        .route("/orders/{id}", get(get_order_admin))
}

#[utoipa::path(
    get,
    path = "/admin/orders",
    responses(
    (status = 200, description = "Get all orders (admin only)", body = ApiResponse<OrderList>),
    (status = 403, description = "Forbidden"),
    (status = 500, description = "Internal Server Error"),
    ),
    tag = "Admin"
)]
pub async fn list_all_orders(
    State(pool): State<DbPool>,
    user: AuthUser,
) -> AppResult<Json<ApiResponse<OrderList>>> {
    ensure_admin(&user)?;
    let orders = sqlx::query_as::<_, Order>("SELECT * FROM orders")
        .fetch_all(&pool)
        .await?;
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
        .fetch_one(&pool)
        .await?;
    let meta = Meta::new(1, total.0, total.0);

    let order_list = OrderList { items: orders };

    Ok(Json(ApiResponse::success("Orders", order_list, Some(meta))))
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
    tag = "Admin"

)]
pub async fn get_order_admin(
    State(pool): State<DbPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<OrderWithItems>>> {
    ensure_admin(&user)?;
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?;
    let order = match order {
        Some(o) => o,
        None => return Err(AppError::NotFound),
    };

    let items = sqlx::query_as::<_, OrderItem>("SELECT * FROM order_items WHERE order_id = $1")
        .bind(order.id)
        .fetch_all(&pool)
        .await?;

    let data = OrderWithItems { order, items };
    Ok(Json(ApiResponse::success(
        "Order found",
        data,
        Some(Meta::empty()),
    )))
}
