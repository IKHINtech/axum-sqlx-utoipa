use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    audit::log_audit,
    db::DbPool,
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Order, OrderItem},
    response::{ApiResponse, Meta},
    routes::params::{OrderListQuery, SortOrder},
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
    State(db): State<DbPool>,
    user: AuthUser,
    Query(query): Query<OrderListQuery>,
) -> AppResult<Json<ApiResponse<OrderList>>> {
    let (page, limit, offset) = query.pagination.normalize();
    let mut list_builder = QueryBuilder::new("SELECT * FROM orders WHERE user_id = ");
    list_builder.push_bind(user.user_id);
    let mut count_builder = QueryBuilder::new("SELECT COUNT(*) FROM orders WHERE user_id = ");
    count_builder.push_bind(user.user_id);

    if let Some(status) = query.status.as_ref().filter(|s| !s.is_empty()) {
        list_builder.push(" AND status = ").push_bind(status);
        count_builder.push(" AND status = ").push_bind(status);
    }

    let sort_order = query.sort_order.unwrap_or(SortOrder::Desc);
    list_builder
        .push(" ORDER BY created_at ")
        .push(sort_order.as_sql())
        .push(" LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let orders = list_builder
        .build_query_as::<Order>()
        .fetch_all(&db)
        .await?;

    let total: (i64,) = count_builder.build_query_as().fetch_one(&db).await?;

    let meta = Meta::new(page, limit, total.0);
    let data = OrderList { items: orders };
    Ok(Json(ApiResponse::success("Ok", data, Some(meta))))
}

#[derive(sqlx::FromRow)]
pub struct CartProductRow {
    product_id: Uuid,
    quantity: i32,
    price: i64,
    stock: i32,
}
#[utoipa::path(
    post,
    path = "/api/orders/checkout",
    responses(
        (status = 200, description = "Checkout current cart into an order", body = ApiResponse<OrderWithItems>),
        (status = 400, description = "Cart empty or validation error"),
    )
    , security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn checkout(
    State(pool): State<DbPool>,
    user: AuthUser,
) -> AppResult<Json<ApiResponse<OrderWithItems>>> {
    let mut tx = pool.begin().await?;

    // ambil cart + info produk untuk user ini
    let rows = sqlx::query_as::<_, CartProductRow>(
        r#"
        SELECT ci.product_id, ci.quantity, p.price, p.stock
        FROM cart_items ci
        JOIN products p ON p.id = ci.product_id
        WHERE ci.user_id = $1
        FOR UPDATE
        "#,
    )
    .bind(user.user_id)
    .fetch_all(&mut *tx)
    .await?;

    if rows.is_empty() {
        return Err(AppError::BadRequest("Cart is empty".into()));
    }

    // cek stok & hitung total
    let mut total_amount: i64 = 0;
    for row in &rows {
        if row.quantity <= 0 {
            return Err(AppError::BadRequest("Cart has invalid quantity".into()));
        }
        if row.stock < row.quantity {
            return Err(AppError::BadRequest(format!(
                "Insufficient stock for product {}",
                row.product_id
            )));
        }
        total_amount += row.price * (row.quantity as i64);
    }

    let order_id = Uuid::new_v4();
    let invoice_number = build_invoice_number(order_id);

    // insert order
    let order = sqlx::query_as::<_, Order>(
        r#"
        INSERT INTO orders (id, user_id, total_amount, status, payment_status, invoice_number)
        VALUES ($1, $2, $3, 'pending', 'unpaid', $4)
        RETURNING *
        "#,
    )
    .bind(order_id)
    .bind(user.user_id)
    .bind(total_amount)
    .bind(invoice_number)
    .fetch_one(&mut *tx)
    .await?;

    // insert order items & update stok
    let mut order_items: Vec<OrderItem> = Vec::new();

    for row in &rows {
        let item_id = Uuid::new_v4();

        let item = sqlx::query_as::<_, OrderItem>(
            r#"
            INSERT INTO order_items (id, order_id, product_id, quantity, price)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(item_id)
        .bind(order.id)
        .bind(row.product_id)
        .bind(row.quantity)
        .bind(row.price)
        .fetch_one(&mut *tx)
        .await?;

        order_items.push(item);

        // kurangi stok produk
        sqlx::query(
            r#"
            UPDATE products
            SET stock = stock - $2
            WHERE id = $1
            "#,
        )
        .bind(row.product_id)
        .bind(row.quantity)
        .execute(&mut *tx)
        .await?;
    }

    // kosongkan cart user
    sqlx::query("DELETE FROM cart_items WHERE user_id = $1")
        .bind(user.user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    if let Err(err) = log_audit(
        &pool,
        Some(user.user_id),
        "checkout",
        Some("orders"),
        Some(serde_json::json!({ "order_id": order.id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    let data = OrderWithItems {
        order,
        items: order_items,
    };

    Ok(Json(ApiResponse::success(
        "Checkout success",
        data,
        Some(Meta::empty()),
    )))
}

#[utoipa::path(
    post,
    path = "/api/orders/{id}/pay",
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Mark order as paid", body = ApiResponse<OrderWithItems>),
        (status = 400, description = "Invalid order state"),
        (status = 404, description = "Order not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn pay_order(
    State(pool): State<DbPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<OrderWithItems>>> {
    let mut tx = pool.begin().await?;

    let order = sqlx::query_as::<_, Order>(
        "SELECT * FROM orders WHERE user_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(user.user_id)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let mut order = match order {
        Some(o) => o,
        None => return Err(AppError::NotFound),
    };

    if order.payment_status == "paid" {
        return Err(AppError::BadRequest("Order already paid".into()));
    }

    order = sqlx::query_as::<_, Order>(
        r#"
        UPDATE orders
        SET payment_status = 'paid',
            status = 'paid',
            paid_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(order.id)
    .fetch_one(&mut *tx)
    .await?;

    let items = sqlx::query_as::<_, OrderItem>("SELECT * FROM order_items WHERE order_id = $1")
        .bind(order.id)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    if let Err(err) = log_audit(
        &pool,
        Some(user.user_id),
        "order_paid",
        Some("orders"),
        Some(serde_json::json!({ "order_id": order.id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    let data = OrderWithItems { order, items };
    Ok(Json(ApiResponse::success(
        "Payment recorded",
        data,
        Some(Meta::empty()),
    )))
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

fn build_invoice_number(order_id: Uuid) -> String {
    let date = Utc::now().format("%Y%m%d");
    let suffix = order_id.to_string();
    let short = &suffix[..8];
    format!("INV-{}-{}", date, short)
}
