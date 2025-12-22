use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::DbPool,
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Order, OrderItem, Product},
    response::{ApiResponse, Meta},
};

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct OrderList {
    pub items: Vec<Order>,
}

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct OrderWithItems {
    pub order: Order,
    pub items: Vec<OrderItem>,
}

pub fn route() -> Router<DbPool> {
    Router::new()
        .route("/", get(list_order))
        .route("/checkout", post(checkout))
        .route("/{id}", get(get_order))
}

#[utoipa::path(get, path = "/orders", tag = "Orders")]
pub async fn list_order(
    State(db): State<DbPool>,
    user: AuthUser,
) -> AppResult<Json<ApiResponse<OrderList>>> {
    let orders = sqlx::query_as::<_, Order>(
        "SELECT * FROM orders where user_id = $1 order by created_at desc",
    )
    .bind(user.user_id)
    .fetch_all(&db)
    .await?;

    let total: (i64,) =
        sqlx::query_as("SELECT count(*), sum(total) FROM orders where user_id = $1")
            .bind(user.user_id)
            .fetch_one(&db)
            .await?;

    let meta = Meta::new(1, total.0, total.0);
    let data = OrderList { items: orders };
    Ok(Json(ApiResponse::success("Ok", data, Some(meta))))
}
#[utoipa::path(post, path = "/orders/checkout", tag = "Orders")]
pub async fn checkout() -> String {
    "checkout".to_string()
}

#[utoipa::path(get, path = "/orders/{id}", tag = "Orders")]
pub async fn get_order(
    State(db): State<DbPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<OrderWithItems>>> {
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders where user_id = $1 and id = $2")
        .bind(user.user_id)
        .bind(id)
        .fetch_optional(&db)
        .await?;
    let order = match order {
        Some(o) => o,
        None => return Err(AppError::NotFound),
    };

    let items = sqlx::query_as::<_, OrderItem>("SELECT * FROM order_items WHERE order_id = $1")
        .bind(order.id)
        .fetch_all(&db)
        .await?;

    let data = OrderWithItems { order, items };

    Ok(Json(ApiResponse::success("OK", data, Some(Meta::empty()))))
}
