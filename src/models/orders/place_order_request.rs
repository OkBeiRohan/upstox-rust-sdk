use {
    crate::models::{
        orders::{OrderType, ValidityType},
        validate_market_protection, ProductType, TransactionType,
    },
    serde::Serialize,
    serde_valid::Validate,
};

#[derive(Serialize, Debug, Validate)]
pub struct PlaceOrderRequest {
    // For commodity - number of lots is accepted. For other Futures & Options and equities - number of units is accepted in multiples of the tick size.
    #[validate(exclusive_minimum = 0, message = "quantity must be greater than 0")]
    pub quantity: u32,
    pub product: ProductType,
    pub validity: ValidityType,
    #[validate(exclusive_minimum = 0.0, message = "price must be greater than 0.0")]
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[validate(
        pattern = r"^(?:^NSE_EQ|NSE_FO|NSE_COM|NCD_FO|NCD_COM|BSE_EQ|BSE_FO|BSE_COM|BCD_FO|MCX_FO|MCX_COM|NSE_INDEX|BSE_INDEX|MCX_INDEX)\|[\w ]+(,(?:NSE_EQ|NSE_FO|NSE_COM|NCD_FO|NCD_COM|BSE_EQ|BSE_FO|BSE_COM|BCD_FO|MCX_FO|MCX_COM|NSE_INDEX|BSE_INDEX|MCX_INDEX)\|[\w ]+)*?$",
        message = "Invalid instrument_token"
    )]
    pub instrument_token: String,
    pub order_type: OrderType,
    pub transaction_type: TransactionType,
    pub disclosed_quantity: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<f64>,
    pub is_amo: bool,
    /// See [`super::place_order_v3_request::PlaceOrderV3Request::market_protection`].
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = validate_market_protection)]
    pub market_protection: Option<i32>,
}
