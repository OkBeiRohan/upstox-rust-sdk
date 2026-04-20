pub mod charges;
pub mod error_response;
pub mod expired_instruments;
pub mod gtt_orders;
pub mod historical_data;
pub mod instruments;
pub mod login;
pub mod margins;
pub mod market_information;
pub mod market_quote;
pub mod option_chain;
pub mod orders;
pub mod portfolio;
pub mod success_response;
pub mod trade_profit_and_loss;
pub mod user;
pub mod ws;

use {
    crate::utils::serde_spaced_lowercase,
    serde::{Deserialize, Deserializer, Serialize, Serializer},
    std::{
        fmt::{self, Display},
        str::FromStr,
    },
};

/// Validator for the `market_protection` field that is optional across
/// the Place / Modify / Multi / ExitAll / GTT order paths (Upstox
/// announcement of 2026-03-11).
///
/// Accepted values:
/// - `None` — field omitted; broker applies its default.
/// - `Some(-1)` — automatic price protection (broker decides).
/// - `Some(0)` — explicitly no protection (Upstox's table includes 0
///   for symmetry but the exchange will reject the naked MARKET /
///   SL-M order downstream).
/// - `Some(1..=25)` — custom protection percent.
///
/// Reference: <https://upstox.com/developer/api-documentation/announcements/market-protection>
pub fn validate_market_protection(
    value: &Option<i32>,
) -> Result<(), serde_valid::validation::Error> {
    match value {
        None => Ok(()),
        Some(-1) => Ok(()),
        Some(v) if (0..=25).contains(v) => Ok(()),
        Some(v) => Err(serde_valid::validation::Error::Custom(format!(
            "market_protection must be -1 (auto), 0 (none), or 1..=25 (percent); got {v}"
        ))),
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseSummary {
    pub total: u32,
    pub payload_error: Option<u32>,
    pub success: u32,
    pub error: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum AssetType {
    COM,
    INDEX,
    EQUITY,
    CUR,
    IRD,
}

#[derive(Deserialize, Serialize, Debug, Eq, Hash, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExchangeSegment {
    NseEq,
    NseIndex,
    NseFo,
    NseCom,
    NcdFo,
    BseEq,
    BseIndex,
    BseFo,
    BcdFo,
    McxFo,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProductType {
    I,
    D,
    CO,
    MTF,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionType {
    Buy,
    Sell,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SegmentType {
    EQ,
    FO,
    COM,
    CD,
    MF,
}

/// An Upstox exchange code.
///
/// Permissive by design: Upstox periodically introduces new exchange
/// codes (e.g. `NSCOM` added in April 2026) and the SDK must decode them
/// without panicking. Unknown values land in [`Exchange::Other`] so the
/// raw string is preserved for diagnostic logging while still round-tripping
/// over the wire.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(from = "String", into = "String")]
pub enum Exchange {
    NSE,
    NFO,
    CDS,
    BSE,
    BFO,
    BCD,
    MCX,
    /// NSE Commodity segment — added to the public API response set in
    /// the April 2026 Upstox docs rollout.
    NSCOM,
    /// Any exchange code not known to this SDK build. The raw string
    /// is preserved verbatim for log output and `to_string()` comparisons.
    Other(String),
}

impl From<String> for Exchange {
    fn from(s: String) -> Self {
        match s.as_str() {
            "NSE" => Exchange::NSE,
            "NFO" => Exchange::NFO,
            "CDS" => Exchange::CDS,
            "BSE" => Exchange::BSE,
            "BFO" => Exchange::BFO,
            "BCD" => Exchange::BCD,
            "MCX" => Exchange::MCX,
            "NSCOM" => Exchange::NSCOM,
            _ => Exchange::Other(s),
        }
    }
}

impl From<Exchange> for String {
    fn from(e: Exchange) -> String {
        match e {
            Exchange::NSE => "NSE".to_string(),
            Exchange::NFO => "NFO".to_string(),
            Exchange::CDS => "CDS".to_string(),
            Exchange::BSE => "BSE".to_string(),
            Exchange::BFO => "BFO".to_string(),
            Exchange::BCD => "BCD".to_string(),
            Exchange::MCX => "MCX".to_string(),
            Exchange::NSCOM => "NSCOM".to_string(),
            Exchange::Other(s) => s,
        }
    }
}

impl Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s: &str = match self {
            Exchange::NSE => "NSE",
            Exchange::NFO => "NFO",
            Exchange::CDS => "CDS",
            Exchange::BSE => "BSE",
            Exchange::BFO => "BFO",
            Exchange::BCD => "BCD",
            Exchange::MCX => "MCX",
            Exchange::NSCOM => "NSCOM",
            Exchange::Other(s) => s,
        };
        write!(f, "{}", s)
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderVariety {
    Simple,
    AMO,
    CO,
    OCO,
}

#[derive(Debug)]
pub enum OrderStatus {
    ValidationPending,
    ModifyPending,
    TriggerPending,
    PutOrderReqReceived,
    ModifyAfterMarketOrderReqReceived,
    CancelledAfterMarketOrder,
    Open,
    Complete,
    ModifyValidationPending,
    AfterMarketOrderReqReceived,
    Modified,
    NotCancelled,
    CancelPending,
    Rejected,
    Cancelled,
    OpenPending,
    NotModified,
}

impl Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s: &str = match self {
            OrderStatus::ValidationPending => "ValidationPending",
            OrderStatus::ModifyPending => "ModifyPending",
            OrderStatus::TriggerPending => "TriggerPending",
            OrderStatus::PutOrderReqReceived => "PutOrderReqReceived",
            OrderStatus::ModifyAfterMarketOrderReqReceived => "ModifyAfterMarketOrderReqReceived",
            OrderStatus::CancelledAfterMarketOrder => "CancelledAfterMarketOrder",
            OrderStatus::Open => "Open",
            OrderStatus::Complete => "Complete",
            OrderStatus::ModifyValidationPending => "ModifyValidationPending",
            OrderStatus::AfterMarketOrderReqReceived => "AfterMarketOrderReqReceived",
            OrderStatus::Modified => "Modified",
            OrderStatus::NotCancelled => "NotCancelled",
            OrderStatus::CancelPending => "CancelPending",
            OrderStatus::Rejected => "Rejected",
            OrderStatus::Cancelled => "Cancelled",
            OrderStatus::OpenPending => "OpenPending",
            OrderStatus::NotModified => "NotModified",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for OrderStatus {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "validation pending" => Ok(OrderStatus::ValidationPending),
            "modify pending" => Ok(OrderStatus::ModifyPending),
            "trigger pending" => Ok(OrderStatus::TriggerPending),
            "put order req received" => Ok(OrderStatus::PutOrderReqReceived),
            "modify after market order req received" => {
                Ok(OrderStatus::ModifyAfterMarketOrderReqReceived)
            }
            "cancelled after market order" => Ok(OrderStatus::CancelledAfterMarketOrder),
            "open" => Ok(OrderStatus::Open),
            "complete" => Ok(OrderStatus::Complete),
            "modify validation pending" => Ok(OrderStatus::ModifyValidationPending),
            "after market order req received" => Ok(OrderStatus::AfterMarketOrderReqReceived),
            "modified" => Ok(OrderStatus::Modified),
            "not cancelled" => Ok(OrderStatus::NotCancelled),
            "cancel pending" => Ok(OrderStatus::CancelPending),
            "rejected" => Ok(OrderStatus::Rejected),
            "cancelled" => Ok(OrderStatus::Cancelled),
            "open pending" => Ok(OrderStatus::OpenPending),
            "not modified" => Ok(OrderStatus::NotModified),
            _ => Err("Invalid order status"),
        }
    }
}

impl Serialize for OrderStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_spaced_lowercase::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for OrderStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_spaced_lowercase::deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_round_trips_known_variants() {
        for name in ["NSE", "NFO", "CDS", "BSE", "BFO", "BCD", "MCX", "NSCOM"] {
            let wire = format!("\"{name}\"");
            let de: Exchange =
                serde_json::from_str(&wire).expect("known Exchange variant deserializes");
            let ser = serde_json::to_string(&de).expect("Exchange serializes");
            assert_eq!(ser, wire, "round-trip drift for {name}");
            assert_eq!(de.to_string(), name, "Display drift for {name}");
        }
    }

    /// Regression for the April 2026 `NSCOM` incident and every future
    /// one: an unrecognised exchange value must NOT panic deserialization.
    #[test]
    fn exchange_preserves_unknown_values_verbatim() {
        let raw = "\"SOMETHING_NEW_TOMORROW\"";
        let de: Exchange = serde_json::from_str(raw).expect("unknown value must not panic");
        assert_eq!(de, Exchange::Other("SOMETHING_NEW_TOMORROW".to_string()));
        // Round-trip preserves the string exactly.
        let ser = serde_json::to_string(&de).unwrap();
        assert_eq!(ser, raw);
        // Display path keeps existing `eq_ignore_ascii_case(...)` consumers
        // working without any SDK-level string manipulation.
        assert_eq!(de.to_string(), "SOMETHING_NEW_TOMORROW");
    }

    #[test]
    fn exchange_other_is_case_sensitive_and_exact() {
        let de: Exchange = serde_json::from_str("\"nse\"").unwrap();
        // Unlike the closed enum, we do NOT coerce the case here —
        // callers decide whether to normalise.
        assert_eq!(de, Exchange::Other("nse".to_string()));
    }
}
