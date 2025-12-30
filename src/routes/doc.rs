use utoipa::{
    Modify, OpenApi,
    openapi::{
        self,
        OpenApi as OpenApiSpec,
        security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    },
};
use utoipa_scalar::{Scalar, Servable};

use crate::{
    dto::{cart::CartList, favorites::FavoriteProductList, orders::{OrderList, OrderWithItems}, products},
    models::{CartItem, Favorite, Order, OrderItem, Product, User},
    response::{ApiResponse, Meta},
    routes::{admin, auth, cart, favorites, health, orders, params, products as product_routes},
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health_check,
        auth::login,
        auth::register,
        cart::cart_list,
        cart::add_to_cart,
        cart::remove_from_cart,
        product_routes::list_products,
        product_routes::create_product,
        product_routes::get_product,
        product_routes::update_product,
        product_routes::delete_product,
        orders::list_order,
        orders::checkout,
        orders::pay_order,
        orders::get_order,
        admin::list_all_orders,
        admin::get_order_admin,
        admin::update_order_status,
        admin::list_low_stock,
        admin::adjust_inventory,
        favorites::add_favorite,
        favorites::remove_favorite,
        favorites::list_favorites
    ),
    components(
        schemas(
            User,
            Product,
            Favorite,
            CartItem,
            Order,
            OrderItem,
            admin::ProductList,
            admin::UpdateOrderStatusRequest,
            admin::InventoryAdjustRequest,
            admin::LowStockQuery,
            CartList,
            FavoriteProductList,
            OrderList,
            OrderWithItems,
            params::Pagination,
            params::ProductQuery,
            params::OrderListQuery,
            products::ProductList,
            Meta,
            ApiResponse<Product>,
            ApiResponse<products::ProductList>,
            ApiResponse<OrderWithItems>,
            ApiResponse<OrderList>,
            ApiResponse<admin::ProductList>
        )
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Health check endpoint"),
        (name = "Products", description = "Product endpoints"),
        (name = "Cart", description = "Cart endpoints"),
        (name = "Orders", description = "Order endpoints"),
        (name = "Admin", description = "Admin endpoints"),
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Favorites", description = "Favorite endpoints"),
    )
)]
pub struct ApiDoc;

pub fn scalar_docs() -> Scalar<OpenApiSpec> {
    Scalar::with_url("/docs", ApiDoc::openapi())
    //.custom_html(SCALAR_HTML)
}
