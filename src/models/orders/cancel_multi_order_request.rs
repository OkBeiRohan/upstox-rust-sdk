use {crate::models::orders::ExchangeSegment, serde::Serialize, serde_valid::Validate};

#[derive(Serialize, Debug, Validate)]
pub struct CancelMultiOrderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment: Option<ExchangeSegment>,
    /// Filter open orders by the caller-supplied order tag (the same
    /// value passed as `tag` in the original `PlaceOrderV3Request`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}
