use uuid::Uuid;

use crate::{
    audit::log_audit,
    dto::cart::{AddToCartRequest, CartItemDto, CartList},
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::Product,
    response::{ApiResponse, Meta},
    routes::params::Pagination,
    state::AppState,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityOrSelect, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use sea_orm::ActiveValue::NotSet;
use crate::entity::{
    cart_items::{ActiveModel as CartActive, Column as CartCol, Entity as CartItems},
    products::{Column as ProdCol, Entity as Products},
};

pub async fn list_cart(
    state: &AppState,
    user: &AuthUser,
    pagination: Pagination,
) -> AppResult<ApiResponse<CartList>> {
    let (page, limit, offset) = pagination.normalize();
    #[derive(Debug, FromQueryResult)]
    struct CartWithProduct {
        #[sea_orm(column_name = "cart_items.id")]
        cart_id: Uuid,
        #[sea_orm(column_name = "cart_items.product_id")]
        product_id: Uuid,
        #[sea_orm(column_name = "cart_items.quantity")]
        quantity: i32,
        #[sea_orm(column_name = "products.name")]
        name: String,
        #[sea_orm(column_name = "products.description")]
        description: Option<String>,
        #[sea_orm(column_name = "products.price")]
        price: i64,
        #[sea_orm(column_name = "products.stock")]
        stock: i32,
        #[sea_orm(column_name = "products.created_at")]
        created_at: sea_orm::prelude::DateTimeWithTimeZone,
    }

    let rows = CartItems::find()
        .select()
        .column_as(CartCol::Id, "cart_items.id")
        .column_as(CartCol::ProductId, "cart_items.product_id")
        .column_as(CartCol::Quantity, "cart_items.quantity")
        .join(sea_orm::JoinType::InnerJoin, CartItems::belongs_to(Products).into())
        .column_as(ProdCol::Name, "products.name")
        .column_as(ProdCol::Description, "products.description")
        .column_as(ProdCol::Price, "products.price")
        .column_as(ProdCol::Stock, "products.stock")
        .column_as(ProdCol::CreatedAt, "products.created_at")
        .filter(CartCol::UserId.eq(user.user_id))
        .order_by_desc(CartCol::CreatedAt)
        .limit(limit as u64)
        .offset(offset as u64)
        .into_model::<CartWithProduct>()
        .all(&state.orm)
        .await?;

    let total = CartItems::find()
        .filter(CartCol::UserId.eq(user.user_id))
        .count(&state.orm)
        .await? as i64;

    let items = rows
        .into_iter()
        .map(|row| CartItemDto {
            id: row.cart_id,
            product: Product {
                id: row.product_id,
                name: row.name,
                description: row.description,
                price: row.price,
                stock: row.stock,
                created_at: row.created_at.with_timezone(&chrono::Utc),
            },
            quantity: row.quantity,
        })
        .collect();

    let meta = Meta::new(page, limit, total);
    Ok(ApiResponse::success("OK", CartList { items }, Some(meta)))
}

pub async fn add_to_cart(
    state: &AppState,
    user: &AuthUser,
    payload: AddToCartRequest,
) -> AppResult<ApiResponse<crate::models::CartItem>> {
    if payload.quantity <= 0 {
        return Err(AppError::BadRequest(
            "quantity must be greater than 0".to_string(),
        ));
    }

    let product_exist = Products::find_by_id(payload.product_id)
        .one(&state.orm)
        .await?;
    if product_exist.is_none() {
        return Err(AppError::BadRequest("product not found".to_string()));
    }

    let exist = CartItems::find()
        .filter(
            Condition::all()
                .add(CartCol::UserId.eq(user.user_id))
                .add(CartCol::ProductId.eq(payload.product_id)),
        )
        .one(&state.orm)
        .await?;

    let cart_item = if let Some(item) = exist {
        let mut active: CartActive = item.into();
        active.quantity = Set(payload.quantity);
        active.update(&state.orm).await?
    } else {
        CartActive {
            id: Set(Uuid::new_v4()),
            user_id: Set(user.user_id),
            product_id: Set(payload.product_id),
            quantity: Set(payload.quantity),
            created_at: NotSet,
        }
        .insert(&state.orm)
        .await?
    };

    if let Err(err) = log_audit(
        state,
        Some(user.user_id),
        "cart_update",
        Some("cart_items"),
        Some(serde_json::json!({ "product_id": payload.product_id, "quantity": payload.quantity })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    // map to API model CartItem (reuse existing struct)
    let api_item = crate::models::CartItem {
        id: cart_item.id,
        product_id: cart_item.product_id,
        user_id: cart_item.user_id,
        quantity: cart_item.quantity,
        created_at: cart_item.created_at.into(),
    };

    Ok(ApiResponse::success("OK", api_item, None))
}

pub async fn remove_from_cart(
    state: &AppState,
    user: &AuthUser,
    product_id: Uuid,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let result = CartItems::delete_many()
        .filter(
            Condition::all()
                .add(CartCol::ProductId.eq(product_id))
                .add(CartCol::UserId.eq(user.user_id)),
        )
        .exec(&state.orm)
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::NotFound);
    }

    if let Err(err) = log_audit(
        state,
        Some(user.user_id),
        "cart_remove",
        Some("cart_items"),
        Some(serde_json::json!({ "product_id": product_id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Removed from cart",
        serde_json::json!({}),
        Some(Meta::empty()),
    ))
}
