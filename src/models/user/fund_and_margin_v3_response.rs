//! Response types for the V3 Get Fund and Margin API.
//!
//! Added in the 2026-04-10 Upstox announcement
//! (<https://upstox.com/developer/api-documentation/announcements/get-funds-and-margin-v3/>).
//!
//! Breaking differences vs the V2 `FundAndMarginResponse`:
//!
//! - No `segment` parameter on the request. The V3 response surfaces
//!   equity + commodity figures together.
//! - Nested `available_to_trade` structure that splits the operator's
//!   buying power into its `cash` and `pledge` components — crucial
//!   for MTF / pledge-collateral workflows.
//! - Separate `unavailable_to_trade` breakdown (used margin, broken
//!   out per segment).

use serde::{Deserialize, Serialize};

/// Cash + pledge components of the amount available to place fresh orders.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AvailableToTrade {
    /// Cash component (free balance + unsettled buy-side credits) usable
    /// against any product type.
    #[serde(default)]
    pub cash: f64,
    /// Pledge component — collateral value usable against MTF and
    /// margin-hungry F&O products.
    #[serde(default)]
    pub pledge: f64,
}

impl AvailableToTrade {
    /// Convenience — total amount the operator can deploy right now,
    /// summing cash and pledge legs. Exposed so callers that haven't
    /// yet migrated to the split view keep a one-liner for "available".
    pub fn total(&self) -> f64 {
        self.cash + self.pledge
    }
}

/// Amount currently tied up in open / unsettled / pledged positions.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct UnavailableToTrade {
    /// Margin used by open positions.
    #[serde(default)]
    pub used_margin: f64,
    /// Unsettled SELL-side credits (T+1 / T+2 settlement window).
    #[serde(default)]
    pub unsettled_credits: f64,
    /// SPAN margin component of the used margin (F&O).
    #[serde(default)]
    pub span_margin: f64,
    /// Exposure margin component of the used margin (F&O).
    #[serde(default)]
    pub exposure_margin: f64,
    /// Adhoc margin extended by the broker.
    #[serde(default)]
    pub adhoc_margin: f64,
}

/// V3 Get-Fund-and-Margin response body.
///
/// Permissive by design — every field carries `#[serde(default)]` so
/// a minor Upstox schema change (new sub-field) doesn't break decode.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FundAndMarginV3Response {
    #[serde(default)]
    pub available_to_trade: AvailableToTrade,
    #[serde(default)]
    pub unavailable_to_trade: UnavailableToTrade,
    /// Combined equity + commodity ledger balance (flat number). Retained
    /// for clients that don't need the cash/pledge split.
    #[serde(default)]
    pub payin_amount: f64,
    /// Unsettled profit/loss that will settle on T+1 / T+2.
    #[serde(default)]
    pub unsettled_profit: f64,
    /// Notional cash (collateral value of pledged holdings) visible to
    /// the segments it backstops.
    #[serde(default)]
    pub notional_cash: f64,
}
