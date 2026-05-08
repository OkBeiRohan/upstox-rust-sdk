use {
    crate::models::Exchange,
    serde::{Deserialize, Serialize},
};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketStatus {
    NormalOpen,
    NormalClose,
    PreOpenStart,
    PreOpenEnd,
    ClosingStart,
    ClosingEnd,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ExchangeStatusResponse {
    pub exchange: Exchange,
    pub status: MarketStatus,
    pub last_updated: u64,
}
