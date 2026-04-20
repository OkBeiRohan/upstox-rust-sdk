use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MethodTypeV3 {
    Sub,
    ChangeMode,
    Unsub,
}

/// Subscription mode for the V3 market-data feed.
///
/// Wire names are pinned per variant via `#[serde(rename)]` to decouple
/// them from serde's rename-all heuristics (specifically to guarantee
/// `FullD30 → "full_d30"` regardless of the heck version serde pulls in).
///
/// Reference: <https://upstox.com/developer/api-documentation/v3/get-market-data-feed>
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeTypeV3 {
    /// Only LTP and close-price changes.
    #[serde(rename = "ltpc")]
    LTPC,
    /// Option greeks only.
    #[serde(rename = "option_greeks")]
    OptionGreeks,
    /// LTPC + 5 market-level quotes + extended metadata + greeks
    /// (proto enum name is `full_d5`).
    #[serde(rename = "full")]
    Full,
    /// LTPC + 30 market-level quotes + extended metadata + greeks.
    /// Upstox Plus only; 50 instrument keys per-connection cap.
    #[serde(rename = "full_d30")]
    FullD30,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MarketDataFeedV3Message {
    pub guid: String,
    pub method: MethodTypeV3,
    pub data: MessageDataV3,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MessageDataV3 {
    pub mode: ModeTypeV3,
    #[serde(rename = "instrumentKeys")]
    pub instrument_keys: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_type_v3_round_trips_every_wire_name() {
        for (mode, expected) in [
            (ModeTypeV3::LTPC, "\"ltpc\""),
            (ModeTypeV3::OptionGreeks, "\"option_greeks\""),
            (ModeTypeV3::Full, "\"full\""),
            (ModeTypeV3::FullD30, "\"full_d30\""),
        ] {
            let ser = serde_json::to_string(&mode).expect("ModeTypeV3 serializes");
            assert_eq!(ser, expected, "wire name drift for {mode:?}");
            let de: ModeTypeV3 =
                serde_json::from_str(expected).expect("ModeTypeV3 deserializes");
            assert_eq!(de, mode, "round-trip drift for {mode:?}");
        }
    }

    #[test]
    fn method_type_v3_round_trips() {
        for (method, expected) in [
            (MethodTypeV3::Sub, "\"sub\""),
            (MethodTypeV3::ChangeMode, "\"change_mode\""),
            (MethodTypeV3::Unsub, "\"unsub\""),
        ] {
            let ser = serde_json::to_string(&method).expect("MethodTypeV3 serializes");
            assert_eq!(ser, expected);
            let de: MethodTypeV3 = serde_json::from_str(expected).unwrap();
            assert_eq!(de, method);
        }
    }

    /// Byte-for-byte snapshot of the `full_d30` subscribe frame so
    /// downstream consumers that historically hand-rolled the JSON
    /// (because `ModeTypeV3::FullD30` didn't exist) can pin the wire
    /// format against this test when they migrate to the native
    /// variant.
    #[test]
    fn full_d30_subscribe_envelope_snapshot() {
        let msg = MarketDataFeedV3Message {
            guid: "d30-sub".to_string(),
            method: MethodTypeV3::Sub,
            data: MessageDataV3 {
                mode: ModeTypeV3::FullD30,
                instrument_keys: vec!["NSE_FO|63412".to_string()],
            },
        };
        let emitted = serde_json::to_string(&msg).expect("envelope serializes");
        assert_eq!(
            emitted,
            r#"{"guid":"d30-sub","method":"sub","data":{"mode":"full_d30","instrumentKeys":["NSE_FO|63412"]}}"#,
        );
    }

    #[test]
    fn unsubscribe_d30_uses_unsub_method() {
        let msg = MarketDataFeedV3Message {
            guid: "d30-unsub".to_string(),
            method: MethodTypeV3::Unsub,
            data: MessageDataV3 {
                mode: ModeTypeV3::FullD30,
                instrument_keys: vec!["NSE_EQ|INE040A01034".to_string()],
            },
        };
        let val: serde_json::Value =
            serde_json::to_value(&msg).expect("envelope -> serde_json::Value");
        assert_eq!(val["method"], "unsub");
        assert_eq!(val["data"]["mode"], "full_d30");
    }
}
