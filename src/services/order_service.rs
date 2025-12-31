use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityOrSelect, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use sea_orm::ActiveValue::NotSet;
use sea_orm::sea_query::LockType;
use sea_orm::sea_query::Expr;
use uuid::Uuid;

use crate::{
    audit::log_audit,
    dto::orders::{CheckoutRequest, OrderList, OrderWithItems, PayOrderRequest},
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Order, OrderItem},
    response::{ApiResponse, Meta},
    routes::params::{OrderListQuery, SortOrder},
    state::AppState,
    entity::{
        cart_items::{Column as CartCol, Entity as CartItems},
        orders::{ActiveModel as OrderActive, Column as OrderCol, Entity as Orders, Model as OrderModel},
        order_items::{ActiveModel as OrderItemActive, Column as OrderItemCol, Entity as OrderItems, Model as OrderItemModel},
        products::{Column as ProdCol, Entity as Products},
    },
};

pub async fn list_orders(
    state: &AppState,
    user: &AuthUser,
    query: OrderListQuery,
) -> AppResult<ApiResponse<OrderList>> {
    let (page, limit, offset) = query.pagination.normalize();
    let mut condition = Condition::all().add(OrderCol::UserId.eq(user.user_id));
    if let Some(status) = query.status.as_ref().filter(|s| !s.is_empty()) {
        condition = condition.add(OrderCol::Status.eq(status.clone()));
    }

    let sort_order = query.sort_order.unwrap_or(SortOrder::Desc);

    let mut finder = Orders::find().filter(condition);
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
    Ok(ApiResponse::success(
        "Ok",
        OrderList { items: orders },
        Some(meta),
    ))
}

pub async fn checkout(
    state: &AppState,
    user: &AuthUser,
    _payload: CheckoutRequest,
) -> AppResult<ApiResponse<OrderWithItems>> {
    let txn = state.orm.begin().await?;

    #[derive(Debug, FromQueryResult)]
    struct CartProductRow {
        #[sea_orm(column_name = "cart_items.product_id")]
        product_id: Uuid,
        #[sea_orm(column_name = "cart_items.quantity")]
        quantity: i32,
        #[sea_orm(column_name = "products.price")]
        price: i64,
        #[sea_orm(column_name = "products.stock")]
        stock: i32,
    }

    let rows = CartItems::find()
        .select()
        .column_as(CartCol::ProductId, "cart_items.product_id")
        .column_as(CartCol::Quantity, "cart_items.quantity")
        .join(sea_orm::JoinType::InnerJoin, CartItems::belongs_to(Products).into())
        .column_as(ProdCol::Price, "products.price")
        .column_as(ProdCol::Stock, "products.stock")
        .filter(CartCol::UserId.eq(user.user_id))
        .lock(LockType::Update)
        .into_model::<CartProductRow>()
        .all(&txn)
        .await?;

    if rows.is_empty() {
        return Err(AppError::BadRequest("Cart is empty".into()));
    }

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

    let order = OrderActive {
        id: Set(order_id),
        user_id: Set(user.user_id),
        total_amount: Set(total_amount),
        status: Set("pending".into()),
        payment_status: Set("unpaid".into()),
        invoice_number: Set(invoice_number),
        paid_at: Set(None),
        created_at: NotSet,
        updated_at: NotSet,
    }
    .insert(&txn)
    .await?;

    let mut order_items: Vec<OrderItem> = Vec::new();

    for row in &rows {
        let item = OrderItemActive {
            id: Set(Uuid::new_v4()),
            order_id: Set(order.id),
            product_id: Set(row.product_id),
            quantity: Set(row.quantity),
            price: Set(row.price),
            created_at: NotSet,
        }
        .insert(&txn)
        .await?;

        order_items.push(order_item_from_entity(item));

        // reduce stock
        Products::update_many()
            .col_expr(ProdCol::Stock, Expr::col(ProdCol::Stock).sub(row.quantity))
            .filter(ProdCol::Id.eq(row.product_id))
            .exec(&txn)
            .await?;
    }

    // clear cart
    CartItems::delete_many()
        .filter(CartCol::UserId.eq(user.user_id))
        .exec(&txn)
        .await?;

    txn.commit().await?;

    if let Err(err) = log_audit(
        &state.pool,
        Some(user.user_id),
        "checkout",
        Some("orders"),
        Some(serde_json::json!({ "order_id": order.id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Checkout success",
        OrderWithItems {
            order: order_from_entity(order),
            items: order_items,
        },
        Some(Meta::empty()),
    ))
}

pub async fn pay_order(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
    _payload: PayOrderRequest,
) -> AppResult<ApiResponse<OrderWithItems>> {
    let txn = state.orm.begin().await?;

    let order = Orders::find()
        .filter(
            Condition::all()
                .add(OrderCol::UserId.eq(user.user_id))
                .add(OrderCol::Id.eq(id)),
        )
        .lock(LockType::Update)
        .one(&txn)
        .await?;
    let order = match order {
        Some(o) => o,
        None => return Err(AppError::NotFound),
    };

    if order.payment_status == "paid" {
        return Err(AppError::BadRequest("Order already paid".into()));
    }

    let mut active: OrderActive = order.into();
    active.payment_status = Set("paid".into());
    active.status = Set("paid".into());
    active.paid_at = Set(Some(Utc::now().into()));
    active.updated_at = Set(Utc::now().into());
    let order = active.update(&txn).await?;

    let items = OrderItems::find()
        .filter(OrderItemCol::OrderId.eq(order.id))
        .all(&txn)
        .await?
        .into_iter()
        .map(order_item_from_entity)
        .collect();

    txn.commit().await?;

    if let Err(err) = log_audit(
        &state.pool,
        Some(user.user_id),
        "order_paid",
        Some("orders"),
        Some(serde_json::json!({ "order_id": order.id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Payment recorded",
        OrderWithItems {
            order: order_from_entity(order),
            items,
        },
        Some(Meta::empty()),
    ))
}

pub async fn get_order(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
) -> AppResult<ApiResponse<OrderWithItems>> {
    let order = Orders::find()
        .filter(
            Condition::all()
                .add(OrderCol::UserId.eq(user.user_id))
                .add(OrderCol::Id.eq(id)),
        )
        .one(&state.orm)
        .await?;
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

    Ok(ApiResponse::success(
        "OK",
        OrderWithItems {
            order: order_from_entity(order),
            items,
        },
        Some(Meta::empty()),
    ))
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

fn build_invoice_number(order_id: Uuid) -> String {
    let date = Utc::now().format("%Y%m%d");
    let suffix = order_id.to_string();
    let short = &suffix[..8];
    format!("INV-{}-{}", date, short)
}
