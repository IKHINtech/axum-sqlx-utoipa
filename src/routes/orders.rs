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

#[derive(sqlx::FromRow)]
pub struct CartProductRow {
    product_id: Uuid,
    quantity: i32,
    price: i64,
    stock: i32,
}
#[utoipa::path(post, path = "/orders/checkout", tag = "Orders")]
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

    // insert order
    let order = sqlx::query_as::<_, Order>(
        r#"
        INSERT INTO orders (id, user_id, total_amount, status)
        VALUES ($1, $2, $3, 'pending')
        RETURNING *
        "#,
    )
    .bind(order_id)
    .bind(user.user_id)
    .bind(total_amount)
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
