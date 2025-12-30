use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::Product;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AddFavoriteRequest {
    pub product_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(transparent)]
pub struct FavoriteProductList {
    #[schema(value_type= Vec<Product>)]
    pub items: Vec<Product>,
}
