use uuid::Uuid;

use crate::{
    audit::log_audit,
    dto::favorites::{AddFavoriteRequest, FavoriteProductList},
    entity::{
        favorites::{
            ActiveModel as FavoriteActive, Column as FavCol, Entity as Favorites,
            Model as FavoriteModel,
        },
        products::{Column as ProdCol, Entity as Products},
    },
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Favorite, Product},
    response::{ApiResponse, Meta},
    routes::params::Pagination,
    state::AppState,
};
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityOrSelect, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

pub async fn list_favorites(
    state: &AppState,
    user: &AuthUser,
    pagination: Pagination,
) -> AppResult<ApiResponse<FavoriteProductList>> {
    let (page, limit, offset) = pagination.normalize();
    #[derive(Debug, FromQueryResult)]
    struct FavWithProduct {
        #[sea_orm(column_name = "favorites.product_id")]
        product_id: Uuid,
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

    let rows = Favorites::find()
        .select()
        .column_as(FavCol::ProductId, "favorites.product_id")
        .join(
            sea_orm::JoinType::InnerJoin,
            Favorites::belongs_to(Products).into(),
        )
        .column_as(ProdCol::Name, "products.name")
        .column_as(ProdCol::Description, "products.description")
        .column_as(ProdCol::Price, "products.price")
        .column_as(ProdCol::Stock, "products.stock")
        .column_as(ProdCol::CreatedAt, "products.created_at")
        .filter(FavCol::UserId.eq(user.user_id))
        .order_by_desc(FavCol::CreatedAt)
        .limit(limit as u64)
        .offset(offset as u64)
        .into_model::<FavWithProduct>()
        .all(&state.orm)
        .await?;

    let total = Favorites::find()
        .filter(FavCol::UserId.eq(user.user_id))
        .count(&state.orm)
        .await? as i64;

    let products = rows
        .into_iter()
        .map(|row| Product {
            id: row.product_id,
            name: row.name,
            description: row.description,
            price: row.price,
            stock: row.stock,
            created_at: row.created_at.with_timezone(&chrono::Utc),
        })
        .collect();

    let meta = Meta::new(page, limit, total);
    let data = FavoriteProductList { items: products };
    Ok(ApiResponse::success("OK", data, Some(meta)))
}

pub async fn add_favorite(
    state: &AppState,
    user: &AuthUser,
    payload: AddFavoriteRequest,
) -> AppResult<ApiResponse<Favorite>> {
    let product_exists = Products::find_by_id(payload.product_id)
        .one(&state.orm)
        .await?;
    if product_exists.is_none() {
        return Err(AppError::BadRequest("Product not found".into()));
    }

    let existing = Favorites::find()
        .filter(
            Condition::all()
                .add(FavCol::UserId.eq(user.user_id))
                .add(FavCol::ProductId.eq(payload.product_id)),
        )
        .one(&state.orm)
        .await?;

    let favorite: FavoriteModel = if let Some(fav) = existing {
        fav
    } else {
        FavoriteActive {
            id: Set(Uuid::new_v4()),
            user_id: Set(user.user_id),
            product_id: Set(payload.product_id),
            created_at: NotSet,
        }
        .insert(&state.orm)
        .await?
    };

    if let Err(err) = log_audit(
        state,
        Some(user.user_id),
        "favorite_add",
        Some("favorites"),
        Some(serde_json::json!({ "product_id": payload.product_id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Added to favorites",
        favorite_from_entity(favorite),
        Some(Meta::empty()),
    ))
}

pub async fn remove_favorite(
    state: &AppState,
    user: &AuthUser,
    product_id: Uuid,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let result = Favorites::delete_many()
        .filter(
            Condition::all()
                .add(FavCol::UserId.eq(user.user_id))
                .add(FavCol::ProductId.eq(product_id)),
        )
        .exec(&state.orm)
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::NotFound);
    }

    if let Err(err) = log_audit(
        state,
        Some(user.user_id),
        "favorite_remove",
        Some("favorites"),
        Some(serde_json::json!({ "product_id": product_id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Removed from favorites",
        serde_json::json!({}),
        Some(Meta::empty()),
    ))
}

fn favorite_from_entity(model: FavoriteModel) -> Favorite {
    Favorite {
        id: model.id,
        user_id: model.user_id,
        product_id: model.product_id,
        created_at: model.created_at.into(),
    }
}
