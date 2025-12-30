use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get},
};
use uuid::Uuid;

use crate::{
    error::AppResult,
    middleware::auth::AuthUser,
    models::CartItem,
    response::ApiResponse,
    routes::params::Pagination,
    dto::cart::{AddToCartRequest, CartList},
    services::cart_service,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(cart_list).post(add_to_cart))
        .route("/{product_id}", delete(remove_from_cart))
}

#[utoipa::path(
    get,
    path = "/api/cart",
    params(
        ("page" = Option<i64>, Query, description = "Page number, default 1"),
        ("per_page" = Option<i64>, Query, description = "Items per page, default 20")
    ),
    responses(
        (status = 200, description = "List cart items for current user", body = ApiResponse<CartList>)
    ),
    security(("bearer_auth" = [])),
    tag = "Cart"
)]
pub async fn cart_list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<ApiResponse<CartList>>> {
    let resp = cart_service::list_cart(&state.pool, &user, pagination).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/cart",
    request_body = AddToCartRequest,
    responses(
        (status = 200, description = "Add or update cart item", body = ApiResponse<CartItem>),
        (status = 400, description = "Bad request"),
    ),
    security(("bearer_auth" = [])),
    tag = "Cart"
)]
pub async fn add_to_cart(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<AddToCartRequest>,
) -> AppResult<Json<ApiResponse<CartItem>>> {
    let resp = cart_service::add_to_cart(&state.pool, &user, payload).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    delete,
    path = "/api/cart/{product_id}",
    params(

        ("product_id" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "OK", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Cart item not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Cart"
)]
pub async fn remove_from_cart(
    State(state): State<AppState>,
    auht: AuthUser,
    Path(product_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let resp = cart_service::remove_from_cart(&state.pool, &auht, product_id).await?;
    Ok(Json(resp))
}
