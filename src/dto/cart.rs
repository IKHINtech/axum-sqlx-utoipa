use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::Product;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddToCartRequest {
    pub product_id: Uuid,
    pub quantity: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CartList {
    pub items: Vec<CartItemDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CartItemDto {
    pub id: Uuid,
    pub product: Product,
    pub quantity: i32,
}
