use {
    crate::models::{ProductType, TransactionType, orders::OrderType},
    serde::{Deserialize, Serialize},
};

#[derive(Deserialize, Serialize, Debug)]
pub struct TradeDetailsResponse {
    pub exchange: String,
    pub product: ProductType,
    pub trading_symbol: String,
    pub instrument_token: String,
    pub order_type: OrderType,
    pub transaction_type: TransactionType,
    pub quantity: u32,
    pub price: f64,
    pub exchange_order_id: String,
    pub order_id: String,
    pub exchange_timestamp: String,
    pub average_price: f64,
    pub trade_id: String,
    pub order_ref_id: String,
    pub order_timestamp: String,
}
