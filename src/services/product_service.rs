use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::{
    audit::log_audit,
    db::DbPool,
    error::{AppError, AppResult},
    middleware::auth::{AuthUser, ensure_admin},
    models::Product,
    response::{ApiResponse, Meta},
    routes::params::{ProductQuery, ProductSortBy, SortOrder},
};
use crate::dto::products::{CreateProductRequest, ProductList, UpdateProductRequest};

pub async fn list_products(
    pool: &DbPool,
    query: ProductQuery,
) -> AppResult<ApiResponse<ProductList>> {
    let (page, limit, offset) = query.pagination.normalize();
    let mut list_builder = QueryBuilder::new("SELECT * FROM products");
    let mut count_builder = QueryBuilder::new("SELECT COUNT(*) FROM products");
    let mut has_where = false;
    let has_max_price = query.max_price.is_some();
    let needs_price_filter = query.min_price.is_some() || query.max_price.is_some();

    if let Some(search) = query.q.as_ref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", search);
        let clause = " WHERE (name ILIKE ";
        list_builder
            .push(clause)
            .push_bind(pattern.clone());
        list_builder
            .push(" OR COALESCE(description, '') ILIKE ")
            .push_bind(pattern.clone())
            .push(")");
        count_builder
            .push(clause)
            .push_bind(pattern.clone());
        count_builder
            .push(" OR COALESCE(description, '') ILIKE ")
            .push_bind(pattern)
            .push(")");
        has_where = needs_price_filter;
    }

    if let Some(min_price) = query.min_price {
        let clause = if has_where { " AND " } else { " WHERE " };
        list_builder
            .push(clause)
            .push("price >= ")
            .push_bind(min_price);
        count_builder
            .push(clause)
            .push("price >= ")
            .push_bind(min_price);
        if has_max_price {
            has_where = true;
        }
    }

    if let Some(max_price) = query.max_price {
        let clause = if has_where { " AND " } else { " WHERE " };
        list_builder
            .push(clause)
            .push("price <= ")
            .push_bind(max_price);
        count_builder
            .push(clause)
            .push("price <= ")
            .push_bind(max_price);
    }

    let sort_by = query.sort_by.unwrap_or(ProductSortBy::CreatedAt);
    let sort_order = query.sort_order.unwrap_or(SortOrder::Desc);
    list_builder
        .push(" ORDER BY ")
        .push(sort_by.as_sql())
        .push(" ")
        .push(sort_order.as_sql())
        .push(" LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let items = list_builder
        .build_query_as::<Product>()
        .fetch_all(pool)
        .await?;

    let total: (i64,) = count_builder.build_query_as().fetch_one(pool).await?;

    let meta = Meta::new(page, limit, total.0);
    let data = ProductList { items };
    Ok(ApiResponse::success("Products", data, Some(meta)))
}

pub async fn get_product(pool: &DbPool, id: Uuid) -> AppResult<ApiResponse<Product>> {
    let result = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    let result = match result {
        Some(p) => p,
        None => return Err(AppError::NotFound),
    };
    Ok(ApiResponse::success("Product", result, None))
}

pub async fn create_product(
    pool: &DbPool,
    user: &AuthUser,
    payload: CreateProductRequest,
) -> AppResult<ApiResponse<Product>> {
    ensure_admin(user)?;
    let id = Uuid::new_v4();
    let product = sqlx::query_as::<_, Product>(
        "INSERT INTO products (id, name, description, price, stock) VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(id)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.price)
    .bind(payload.stock)
    .fetch_one(pool)
    .await?;

    if let Err(err) = log_audit(
        pool,
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
        product,
        Some(Meta::empty()),
    ))
}

pub async fn update_product(
    pool: &DbPool,
    user: &AuthUser,
    id: Uuid,
    payload: UpdateProductRequest,
) -> AppResult<ApiResponse<Product>> {
    ensure_admin(user)?;
    let existing = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    let existing = match existing {
        Some(p) => p,
        None => return Err(AppError::NotFound),
    };

    let name = payload.name.unwrap_or(existing.name);
    let description = payload.description.or(existing.description);
    let price = payload.price.unwrap_or(existing.price);
    let stock = payload.stock.unwrap_or(existing.stock);

    let product = sqlx::query_as::<_, Product>(
        r#"
        UPDATE products
        SET name = $2, description = $3, price = $4, stock = $5
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(price)
    .bind(stock)
    .fetch_one(pool)
    .await?;

    if let Err(err) = log_audit(
        pool,
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
        product,
        Some(Meta::empty()),
    ))
}

pub async fn delete_product(
    pool: &DbPool,
    user: &AuthUser,
    id: Uuid,
) -> AppResult<ApiResponse<serde_json::Value>> {
    ensure_admin(user)?;
    let result = sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    if let Err(err) = log_audit(
        pool,
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
