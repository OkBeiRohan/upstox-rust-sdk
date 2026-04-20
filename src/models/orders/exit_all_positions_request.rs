use {
    crate::models::{orders::ExchangeSegment, validate_market_protection},
    serde::Serialize,
    serde_valid::Validate,
};

#[derive(Serialize, Debug, Validate)]
pub struct ExitAllPositionsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment: Option<ExchangeSegment>,
    /// Filter open positions by the caller-supplied order tag (the
    /// same value passed as `tag` in the original `PlaceOrderV3Request`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// See [`super::place_order_v3_request::PlaceOrderV3Request::market_protection`].
    /// Upstox applies MPP automatically on this endpoint; this field is
    /// exposed for operators who want to pin a specific percent.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = validate_market_protection)]
    pub market_protection: Option<i32>,
}
