use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::models::{Order, OrderItem};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CheckoutRequest {
    pub address: String,
    pub payment_method: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PayOrderRequest {
    pub invoice_number: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderWithItems {
    pub order: Order,
    pub items: Vec<OrderItem>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderList {
    pub items: Vec<Order>,
}
