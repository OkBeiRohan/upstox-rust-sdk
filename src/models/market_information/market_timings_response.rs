use {
    crate::models::Exchange,
    serde::{Deserialize, Serialize},
};

#[derive(Deserialize, Serialize, Debug)]
pub struct MarketTimingResponse {
    pub exchange: Exchange,
    pub start_time: u64,
    pub end_time: u64,
}
