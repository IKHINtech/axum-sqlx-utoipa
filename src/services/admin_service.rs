use uuid::Uuid;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use sea_orm::sea_query::LockType;
use sea_orm::ActiveValue::Set;

use crate::{
    audit::log_audit,
    entity::{
        order_items::{Column as OrderItemCol, Entity as OrderItems, Model as OrderItemModel},
        orders::{ActiveModel as OrderActive, Column as OrderCol, Entity as Orders, Model as OrderModel},
        products::{ActiveModel as ProductActive, Column as ProdCol, Entity as Products, Model as ProductModel},
    },
    dto::orders::OrderWithItems,
    error::{AppError, AppResult},
    middleware::auth::{AuthUser, ensure_admin},
    models::{Order, OrderItem, Product},
    response::{ApiResponse, Meta},
    routes::params::{OrderListQuery, SortOrder},
    routes::admin::{InventoryAdjustRequest, LowStockQuery, ProductList, UpdateOrderStatusRequest},
    dto::orders::OrderList,
    state::AppState,
};

pub async fn list_all_orders(
    state: &AppState,
    user: &AuthUser,
    query: OrderListQuery,
) -> AppResult<ApiResponse<OrderList>> {
    ensure_admin(user)?;
    let (page, limit, offset) = query.pagination.normalize();

    let mut condition = Condition::all();
    if let Some(status) = query.status.as_ref().filter(|s| !s.is_empty()) {
        condition = condition.add(OrderCol::Status.eq(status.clone()));
    }

    let mut finder = Orders::find().filter(condition);

    let sort_order = query.sort_order.unwrap_or(SortOrder::Desc);
    finder = match sort_order {
        SortOrder::Asc => finder.order_by_asc(OrderCol::CreatedAt),
        SortOrder::Desc => finder.order_by_desc(OrderCol::CreatedAt),
    };

    let total = finder.clone().count(&state.orm).await? as i64;

    let orders = finder
        .limit(limit as u64)
        .offset(offset as u64)
        .all(&state.orm)
        .await?
        .into_iter()
        .map(order_from_entity)
        .collect();

    let meta = Meta::new(page, limit, total);

    let order_list = OrderList { items: orders };

    Ok(ApiResponse::success("Orders", order_list, Some(meta)))
}

pub async fn get_order_admin(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
) -> AppResult<ApiResponse<OrderWithItems>> {
    ensure_admin(user)?;
    let order = Orders::find_by_id(id)
        .one(&state.orm)
        .await?
        .map(order_from_entity);
    let order = match order {
        Some(o) => o,
        None => return Err(AppError::NotFound),
    };

    let items = OrderItems::find()
        .filter(OrderItemCol::OrderId.eq(order.id))
        .all(&state.orm)
        .await?
        .into_iter()
        .map(order_item_from_entity)
        .collect();

    let data = OrderWithItems { order, items };
    Ok(ApiResponse::success(
        "Order found",
        data,
        Some(Meta::empty()),
    ))
}

pub async fn update_order_status(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
    payload: UpdateOrderStatusRequest,
) -> AppResult<ApiResponse<Order>> {
    ensure_admin(user)?;
    validate_order_status(&payload.status)?;

    let existing = Orders::find_by_id(id).one(&state.orm).await?;
    let existing = match existing {
        Some(o) => o,
        None => return Err(AppError::NotFound),
    };

    let mut active: OrderActive = existing.into();
    active.status = Set(payload.status);
    active.updated_at = Set(Utc::now().into());
    let order = active.update(&state.orm).await?;

    if let Err(err) = log_audit(
        state,
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
        order_from_entity(order),
        Some(Meta::empty()),
    ))
}

pub async fn list_low_stock(
    state: &AppState,
    user: &AuthUser,
    query: LowStockQuery,
) -> AppResult<ApiResponse<ProductList>> {
    ensure_admin(user)?;
    let threshold = query.threshold.unwrap_or(5);
    let (page, limit, offset) = query.pagination.normalize();

    let mut finder = Products::find().filter(ProdCol::Stock.lte(threshold));
    finder = finder
        .order_by_asc(ProdCol::Stock)
        .order_by_desc(ProdCol::CreatedAt);

    let total = finder.clone().count(&state.orm).await? as i64;

    let items = finder
        .limit(limit as u64)
        .offset(offset as u64)
        .all(&state.orm)
        .await?
        .into_iter()
        .map(product_from_entity)
        .collect();

    let data = ProductList { items };
    let meta = Meta::new(page, limit, total);
    Ok(ApiResponse::success("Low stock", data, Some(meta)))
}

pub async fn adjust_inventory(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
    payload: InventoryAdjustRequest,
) -> AppResult<ApiResponse<Product>> {
    ensure_admin(user)?;
    if payload.delta == 0 {
        return Err(AppError::BadRequest("delta must not be 0".into()));
    }

    let txn = state.orm.begin().await?;
    let product = Products::find_by_id(id)
        .lock(LockType::Update)
        .one(&txn)
        .await?;
    let product = match product {
        Some(p) => p,
        None => return Err(AppError::NotFound),
    };

    let new_stock = product.stock + payload.delta;
    if new_stock < 0 {
        return Err(AppError::BadRequest("stock cannot be negative".into()));
    }

    let mut active: ProductActive = product.into();
    active.stock = Set(new_stock);
    let updated = active.update(&txn).await?;

    txn.commit().await?;

    if let Err(err) = log_audit(
        state,
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
        product_from_entity(updated),
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

fn order_from_entity(model: OrderModel) -> Order {
    Order {
        id: model.id,
        user_id: model.user_id,
        total_amount: model.total_amount,
        status: model.status,
        payment_status: model.payment_status,
        invoice_number: model.invoice_number,
        paid_at: model.paid_at.map(|dt| dt.with_timezone(&Utc)),
        created_at: model.created_at.with_timezone(&Utc),
        updated_at: model.updated_at.with_timezone(&Utc),
    }
}

fn order_item_from_entity(model: OrderItemModel) -> OrderItem {
    OrderItem {
        id: model.id,
        order_id: model.order_id,
        product_id: model.product_id,
        quantity: model.quantity,
        price: model.price,
        created_at: model.created_at.with_timezone(&Utc),
    }
}

fn product_from_entity(model: ProductModel) -> Product {
    Product {
        id: model.id,
        name: model.name,
        description: model.description,
        price: model.price,
        stock: model.stock,
        created_at: model.created_at.with_timezone(&Utc),
    }
}
