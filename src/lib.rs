//! A Rust client for communicating with the [Upstox API](<https://upstox.com/uplink/>).

//! Upstox API is a set of rest APIs that provide data required to build a complete investment and trading platform. Execute orders in real time, manage user portfolio, stream live market data (using Websockets), and a lot more with this crate.

//! Refer to [`client`] for usage guides.
mod apis;
pub mod client;
pub mod constants;
pub mod models;
pub mod protos;
pub mod rate_limiter;
mod utils;
pub mod ws_client;

// Convenience re-exports so callers can `use upstox_rust_sdk::{WsConnectionRole, ...};`
// without walking the module tree.
pub use client::ClientCapabilities;
pub use rate_limiter::{RateLimitBucket, RateLimitExceeded, RateLimitProfile};
pub use ws_client::{
    ALL_WS_CONNECTION_ROLES, MAX_MARKET_DATA_CONNECTIONS, MAX_MARKET_DATA_CONNECTIONS_STANDARD,
    MarketDataFeedV3CallbackBox, MarketDataFeedV3ClientPool, MarketDataFeedV3EzClient,
    MarketDataV3Call, WsConnectionId, WsConnectionRole,
};
