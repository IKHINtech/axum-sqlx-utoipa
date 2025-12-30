use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get},
};
use uuid::Uuid;

use crate::{
    error::AppResult,
    middleware::auth::AuthUser,
    models::Favorite,
    response::ApiResponse,
    routes::params::Pagination,
    dto::favorites::{AddFavoriteRequest, FavoriteProductList},
    services::favorite_service,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_favorites).post(add_favorite))
        .route("/{product_id}", delete(remove_favorite))
}

#[utoipa::path(
    delete,
    path = "/api/favorites/{product_id}",
    params(
        ("product_id" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Removed from favorites", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Favorite not found")
    ),
    security(("bearer_auth" = [])),
    tag = "Favorites"
)]
pub async fn remove_favorite(
    State(state): State<AppState>,
    user: AuthUser,
    Path(product_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let resp = favorite_service::remove_favorite(&state.pool, &user, product_id).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/favorites",
    params(
        ("page" = Option<i64>, Query, description = "Page number, default 1"),
        ("per_page" = Option<i64>, Query, description = "Items per page, default 20")
    ),
    responses(
        (status = 200, description = "List favorites", body = ApiResponse<FavoriteProductList>)
    ),
    security(("bearer_auth" = [])),
    tag = "Favorites"
)]
pub async fn list_favorites(
    State(state): State<AppState>,
    user: AuthUser,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<ApiResponse<FavoriteProductList>>> {
    let resp = favorite_service::list_favorites(&state.pool, &user, pagination).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/favorites",
    request_body = AddFavoriteRequest,
    responses(
        (status = 200, description = "Added to favorites", body = ApiResponse<Favorite>),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Not Found")
    ),
    security(("bearer_auth" = [])),
    tag = "Favorites"
)]
pub async fn add_favorite(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<AddFavoriteRequest>,
) -> AppResult<Json<ApiResponse<Favorite>>> {
    let resp = favorite_service::add_favorite(&state.pool, &user, payload).await?;
    Ok(Json(resp))
}
