use {
    crate::models::{Exchange, market_information::market_timings_response::MarketTimingResponse},
    serde::{Deserialize, Serialize},
};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HolidayType {
    SettlementHoliday,
    TradingHoliday,
    SpecialTiming,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MarketHolidayResponse {
    pub date: String,
    pub description: String,
    pub holiday_type: HolidayType,
    pub closed_exchanges: Vec<Exchange>,
    pub open_exchanges: Vec<MarketTimingResponse>,
}
