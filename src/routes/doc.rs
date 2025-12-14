use utoipa::{OpenApi, openapi::OpenApi as OpenApiSpec};
use utoipa_scalar::{Scalar, Servable};

use crate::{
    models::{User, Product, Favorite, CartItem, Order, OrderItem},
    response::{ApiResponse, Meta},
    routes::{ products, auth},
    
};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login,
        auth::register,
        products::list_products,
        products::create_product,
        products::get_product,
        products::update_product,
        products::delete_product,
    ),
    components(
        schemas(
            User,
            Product,
            Favorite,
            CartItem,
            Order,
            OrderItem,
            Meta,
            ApiResponse<Product>,
            ApiResponse<products::ProductList>
        )
    ),
    tags(
        (name = "health", description = "Health check endpoint"),
        (name = "products", description = "Product endpoints"),
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
