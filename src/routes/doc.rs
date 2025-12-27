use utoipa::{OpenApi, openapi::OpenApi as OpenApiSpec};
use utoipa_scalar::{Scalar, Servable};

use crate::{
    models::{CartItem, Favorite, Order, OrderItem, Product, User},
    response::{ApiResponse, Meta},
    routes::{admin, auth, cart, favorites, health, orders, params, products},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health_check,
        auth::login,
        auth::register,
        cart::cart_list,
        cart::add_to_cart,
        cart::remove_from_cart,
        products::list_products,
        products::create_product,
        products::get_product,
        products::update_product,
        products::delete_product,
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
            cart::CartList,
            favorites::FavoriteProductList,
            orders::OrderList,
            orders::OrderWithItems,
            params::Pagination,
            params::ProductQuery,
            params::OrderListQuery,
            products::ProductList,
            Meta,
            ApiResponse<Product>,
            ApiResponse<products::ProductList>,
            ApiResponse<orders::OrderWithItems>,
            ApiResponse<orders::OrderList>,
            ApiResponse<admin::ProductList>
        ),
        security_schemes(
            ("bearer_auth" = {
                type = "http",
                scheme = "bearer",
                bearer_format = "JWT"
            })
        )
    ),
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

const SCALAR_HTML: &str = r#"<!doctype html>
<html>
<head>
    <title>API Reference</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
</head>
<body>

<script id="api-reference" type="application/json">
{
  "themeId": "elysiajs",
  "colorMode": "dark",
  "layout": "modern",
  "spec": $spec
}
</script>
<script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>
"#;
