use uuid::Uuid;

use crate::{
    audit::log_audit,
    entity::products::{ActiveModel, Column, Entity as Products, Model as ProductModel},
    state::AppState,
    error::{AppError, AppResult},
    middleware::auth::{AuthUser, ensure_admin},
    models::Product,
    response::{ApiResponse, Meta},
    routes::params::{ProductQuery, ProductSortBy, SortOrder},
};
use crate::dto::products::{CreateProductRequest, ProductList, UpdateProductRequest};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::ActiveValue::NotSet;
use chrono::Utc;

pub async fn list_products(
    state: &AppState,
    query: ProductQuery,
) -> AppResult<ApiResponse<ProductList>> {
    let (page, limit, offset) = query.pagination.normalize();
    let mut condition = Condition::all();

    if let Some(search) = query.q.as_ref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", search);
        condition = condition.add(
            Condition::any()
                .add(Expr::col(Column::Name).ilike(pattern.clone()))
                .add(Expr::col(Column::Description).ilike(pattern)),
        );
    }

    if let Some(min_price) = query.min_price {
        condition = condition.add(Column::Price.gte(min_price));
    }

    if let Some(max_price) = query.max_price {
        condition = condition.add(Column::Price.lte(max_price));
    }

    let sort_by = query.sort_by.unwrap_or(ProductSortBy::CreatedAt);
    let sort_order = query.sort_order.unwrap_or(SortOrder::Desc);
    let sort_col = match sort_by {
        ProductSortBy::CreatedAt => Column::CreatedAt,
        ProductSortBy::Price => Column::Price,
        ProductSortBy::Name => Column::Name,
    };

    let mut finder = Products::find().filter(condition);
    finder = match sort_order {
        SortOrder::Asc => finder.order_by_asc(sort_col),
        SortOrder::Desc => finder.order_by_desc(sort_col),
    };

    let total = finder.clone().count(&state.orm).await? as i64;

    let items = finder
        .limit(limit as u64)
        .offset(offset as u64)
        .all(&state.orm)
        .await?
        .into_iter()
        .map(product_from_entity)
        .collect();

    let meta = Meta::new(page, limit, total);
    let data = ProductList { items };
    Ok(ApiResponse::success("Products", data, Some(meta)))
}

pub async fn get_product(state: &AppState, id: Uuid) -> AppResult<ApiResponse<Product>> {
    let result = Products::find_by_id(id)
        .one(&state.orm)
        .await?
        .map(product_from_entity);
    let result = match result {
        Some(p) => p,
        None => return Err(AppError::NotFound),
    };
    Ok(ApiResponse::success("Product", result, None))
}

pub async fn create_product(
    state: &AppState,
    user: &AuthUser,
    payload: CreateProductRequest,
) -> AppResult<ApiResponse<Product>> {
    ensure_admin(user)?;
    let id = Uuid::new_v4();
    let active = ActiveModel {
        id: Set(id),
        name: Set(payload.name),
        description: Set(Some(payload.description)),
        price: Set(payload.price),
        stock: Set(payload.stock),
        created_at: NotSet,
    };
    let product = active.insert(&state.orm).await?;

    if let Err(err) = log_audit(
        &state.pool,
        Some(user.user_id),
        "product_create",
        Some("products"),
        Some(serde_json::json!({ "product_id": product.id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Product created",
        product_from_entity(product),
        Some(Meta::empty()),
    ))
}

pub async fn update_product(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
    payload: UpdateProductRequest,
) -> AppResult<ApiResponse<Product>> {
    ensure_admin(user)?;
    let existing = Products::find_by_id(id)
        .one(&state.orm)
        .await?;
    let existing = match existing {
        Some(p) => p,
        None => return Err(AppError::NotFound),
    };

    let mut active: ActiveModel = existing.into();
    if let Some(name) = payload.name {
        active.name = Set(name);
    }
    if let Some(description) = payload.description {
        active.description = Set(Some(description));
    }
    if let Some(price) = payload.price {
        active.price = Set(price);
    }
    if let Some(stock) = payload.stock {
        active.stock = Set(stock);
    }

    let product = active.update(&state.orm).await?;

    if let Err(err) = log_audit(
        &state.pool,
        Some(user.user_id),
        "product_update",
        Some("products"),
        Some(serde_json::json!({ "product_id": product.id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Updated",
        product_from_entity(product),
        Some(Meta::empty()),
    ))
}

pub async fn delete_product(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
) -> AppResult<ApiResponse<serde_json::Value>> {
    ensure_admin(user)?;
    let result = Products::delete_by_id(id).exec(&state.orm).await?;

    if result.rows_affected == 0 {
        return Err(AppError::NotFound);
    }

    if let Err(err) = log_audit(
        &state.pool,
        Some(user.user_id),
        "product_delete",
        Some("products"),
        Some(serde_json::json!({ "product_id": id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Deleted",
        serde_json::json!({}),
        Some(Meta::empty()),
    ))
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
