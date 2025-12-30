use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::{
    audit::log_audit,
    db::DbPool,
    dto::orders::OrderWithItems,
    error::{AppError, AppResult},
    middleware::auth::{AuthUser, ensure_admin},
    models::{Order, OrderItem, Product},
    response::{ApiResponse, Meta},
    routes::params::{OrderListQuery, SortOrder},
    routes::admin::{InventoryAdjustRequest, LowStockQuery, ProductList, UpdateOrderStatusRequest},
    dto::orders::OrderList,
};

pub async fn list_all_orders(
    pool: &DbPool,
    user: &AuthUser,
    query: OrderListQuery,
) -> AppResult<ApiResponse<OrderList>> {
    ensure_admin(user)?;
    let (page, limit, offset) = query.pagination.normalize();
    let mut list_builder = QueryBuilder::new("SELECT * FROM orders");
    let mut count_builder = QueryBuilder::new("SELECT COUNT(*) FROM orders");

    if let Some(status) = query.status.as_ref().filter(|s| !s.is_empty()) {
        list_builder.push(" WHERE status = ").push_bind(status);
        count_builder.push(" WHERE status = ").push_bind(status);
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
        .fetch_all(pool)
        .await?;
    let total: (i64,) = count_builder.build_query_as().fetch_one(pool).await?;
    let meta = Meta::new(page, limit, total.0);

    let order_list = OrderList { items: orders };

    Ok(ApiResponse::success("Orders", order_list, Some(meta)))
}

pub async fn get_order_admin(
    pool: &DbPool,
    user: &AuthUser,
    id: Uuid,
) -> AppResult<ApiResponse<OrderWithItems>> {
    ensure_admin(user)?;
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    let order = match order {
        Some(o) => o,
        None => return Err(AppError::NotFound),
    };

    let items = sqlx::query_as::<_, OrderItem>("SELECT * FROM order_items WHERE order_id = $1")
        .bind(order.id)
        .fetch_all(pool)
        .await?;

    let data = OrderWithItems { order, items };
    Ok(ApiResponse::success(
        "Order found",
        data,
        Some(Meta::empty()),
    ))
}

pub async fn update_order_status(
    pool: &DbPool,
    user: &AuthUser,
    id: Uuid,
    payload: UpdateOrderStatusRequest,
) -> AppResult<ApiResponse<Order>> {
    ensure_admin(user)?;
    validate_order_status(&payload.status)?;

    let order = sqlx::query_as::<_, Order>(
        r#"
        UPDATE orders
        SET status = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(payload.status)
    .fetch_optional(pool)
    .await?;

    let order = match order {
        Some(o) => o,
        None => return Err(AppError::NotFound),
    };

    if let Err(err) = log_audit(
        pool,
        Some(user.user_id),
        "order_status_update",
        Some("orders"),
        Some(serde_json::json!({ "order_id": order.id, "status": order.status })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Order updated",
        order,
        Some(Meta::empty()),
    ))
}

pub async fn list_low_stock(
    pool: &DbPool,
    user: &AuthUser,
    query: LowStockQuery,
) -> AppResult<ApiResponse<ProductList>> {
    ensure_admin(user)?;
    let threshold = query.threshold.unwrap_or(5);
    let (page, limit, offset) = query.pagination.normalize();

    let items = sqlx::query_as::<_, Product>(
        "SELECT * FROM products WHERE stock <= $1 ORDER BY stock ASC, created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(threshold)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products WHERE stock <= $1")
        .bind(threshold)
        .fetch_one(pool)
        .await?;

    let data = ProductList { items };
    let meta = Meta::new(page, limit, total.0);
    Ok(ApiResponse::success("Low stock", data, Some(meta)))
}

pub async fn adjust_inventory(
    pool: &DbPool,
    user: &AuthUser,
    id: Uuid,
    payload: InventoryAdjustRequest,
) -> AppResult<ApiResponse<Product>> {
    ensure_admin(user)?;
    if payload.delta == 0 {
        return Err(AppError::BadRequest("delta must not be 0".into()));
    }

    let mut tx = pool.begin().await?;
    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    let product = match product {
        Some(p) => p,
        None => return Err(AppError::NotFound),
    };

    let new_stock = product.stock + payload.delta;
    if new_stock < 0 {
        return Err(AppError::BadRequest("stock cannot be negative".into()));
    }

    let updated =
        sqlx::query_as::<_, Product>("UPDATE products SET stock = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(new_stock)
            .fetch_one(&mut *tx)
            .await?;

    tx.commit().await?;

    if let Err(err) = log_audit(
        pool,
        Some(user.user_id),
        "inventory_adjust",
        Some("products"),
        Some(serde_json::json!({ "product_id": updated.id, "delta": payload.delta })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Inventory updated",
        updated,
        Some(Meta::empty()),
    ))
}

fn validate_order_status(status: &str) -> Result<(), AppError> {
    const VALID: [&str; 5] = ["pending", "paid", "shipped", "completed", "cancelled"];
    if VALID.contains(&status) {
        Ok(())
    } else {
        Err(AppError::BadRequest("Invalid order status".into()))
    }
}
