pub mod cancel_gtt_order_request;
pub mod gtt_order_details_request;
pub mod gtt_order_details_response;
pub mod gtt_orders_response;
pub mod modify_gtt_order_request;
pub mod place_gtt_order_request;

use {
    crate::models::{validate_market_protection, ProductType, TransactionType},
    serde::{Deserialize, Serialize},
    serde_valid::{Validate, validation::Error},
};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum GTTOrderType {
    Single,
    Multiple,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum GTTRuleStrategy {
    Entry,
    Target,
    StopLoss,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum GTTRuleStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum GTTOrderTriggerType {
    Below,
    Above,
    Immediate,
}

#[derive(Serialize, Debug, Validate)]
#[validate(custom = |s| validate_trailing_gap(&s.strategy, &s.trailing_gap))]
pub struct GTTOrderRule {
    pub strategy: GTTRuleStrategy,
    pub trigger_type: GTTOrderTriggerType,
    #[validate(
        exclusive_minimum = 0.0,
        message = "trigger_price must be greater than 0.0"
    )]
    pub trigger_price: f64,
    pub trailing_gap: Option<f64>,
    /// See
    /// [`crate::models::orders::place_order_v3_request::PlaceOrderV3Request::market_protection`].
    /// Applied per-rule by Upstox (2026-03-11 announcement).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = validate_market_protection)]
    pub market_protection: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GTTOrderDetailsRule {
    pub strategy: GTTRuleStrategy,
    pub status: GTTRuleStatus,
    pub trigger_type: GTTOrderTriggerType,
    pub trigger_price: f64,
    pub transaction_type: TransactionType,
    pub message: String,
    pub order_id: Option<String>,
    pub trailing_gap: Option<f64>,
    /// See [`GTTOrderRule::market_protection`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_protection: Option<i32>,
}

fn validate_trailing_gap(
    strategy: &GTTRuleStrategy,
    trailing_gap: &Option<f64>,
) -> Result<(), Error> {
    match strategy {
        GTTRuleStrategy::StopLoss => match trailing_gap {
            Some(_) => Ok(()),
            None => Err(Error::Custom(
                "trailing_gap must be present for StopLoss strategy".to_string(),
            )),
        },
        GTTRuleStrategy::Entry | GTTRuleStrategy::Target => match trailing_gap {
            None => Ok(()),
            Some(_) => Err(Error::Custom(
                "trailing_gap must not be present for Entry or Target strategy".to_string(),
            )),
        },
    }
}

pub(super) fn validate_product_type(product: &ProductType) -> Result<(), Error> {
    match product {
        ProductType::I | ProductType::D | ProductType::MTF => Ok(()),
        _ => Err(Error::Custom(
            "product_type must be I or D or MTF".to_string(),
        )),
    }
}
