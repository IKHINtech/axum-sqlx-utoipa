use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::Product;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: String,
    pub price: i64,
    pub stock: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<i64>,
    pub stock: Option<i32>,
}

#[derive(Serialize, ToSchema)]
#[serde(transparent)]
pub struct ProductList {
    #[schema(value_type = Vec<Product>)]
    pub items: Vec<Product>,
}
