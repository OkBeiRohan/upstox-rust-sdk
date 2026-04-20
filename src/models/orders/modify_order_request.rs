use {
    crate::models::{
        orders::{OrderType, ValidityType},
        validate_market_protection,
    },
    serde::Serialize,
    serde_valid::Validate,
};

#[derive(Serialize, Debug, Validate)]
pub struct ModifyOrderRequest {
    // For commodity - number of lots is accepted. For other Futures & Options and equities - number of units is accepted in multiples of the tick size.
    #[validate(exclusive_minimum = 0, message = "quantity must be greater than 0")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u32>,
    pub validity: ValidityType,
    #[validate(exclusive_minimum = 0.0, message = "price must be greater than 0.0")]
    pub price: f64,
    #[validate(pattern = r"^[-a-zA-Z0-9]+", message = "Invalid order_id")]
    pub order_id: String,
    pub order_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disclosed_quantity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<f64>,
    /// See [`super::place_order_v3_request::PlaceOrderV3Request::market_protection`].
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom(validate_market_protection))]
    pub market_protection: Option<i32>,
}
