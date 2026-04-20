//! Open two Upstox V3 market-data WebSockets on different pool slots
//! and one portfolio stream. Subscribes to a couple of index keys in
//! `Full` mode on slot 0 and a single `full_d30` key on slot 1 so the
//! operator can see each slot's traffic independently.
//!
//! Run with `cargo run --example ws_usage`. Requires a valid Upstox
//! access token (the SDK triggers OAuth if missing).

use {
    dotenvy::dotenv,
    futures::future::join_all,
    std::{collections::HashSet, env},
    tokio::signal,
    upstox_rust_sdk::{
        client::{ApiClient, AutomateLoginConfig, LoginConfig, MailProvider, WSConnectConfig, WsChannelConfig},
        constants::UPLINK_API_KEY_ENV,
        models::ws::{
            market_data_feed_v3_message::{MessageDataV3, ModeTypeV3},
            portfolio_feed_request::PortfolioUpdateType,
            portfolio_feed_response::PortfolioFeedResponse,
        },
        protos::market_data_feed_v3::FeedResponse as MarketDataFeedV3Response,
        ws_client::{MarketDataV3Call, WsConnectionRole},
    },
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let _ = dotenv();

    let portfolio_feed_handler = |data: PortfolioFeedResponse| {
        println!("[portfolio] {data:?}");
    };
    let options_chain_handler = |data: MarketDataFeedV3Response| {
        println!("[options_chain] {data:?}");
    };
    let exec_zone_handler = |data: MarketDataFeedV3Response| {
        println!("[exec_zone_d30] {data:?}");
    };

    let api_key: String = env::var(UPLINK_API_KEY_ENV).unwrap();
    let (fetch_instruments, schedule_refresh_instruments) = (false, false);

    let (api_client, tasks_vec) = ApiClient::new(
        &api_key,
        LoginConfig {
            authorize: true,
            automate_login_config: Some(AutomateLoginConfig {
                automate_login: true,
                schedule_login: false,
                automate_fetching_otp: true,
                mail_provider: Some(MailProvider::Google),
            }),
        },
        fetch_instruments,
        schedule_refresh_instruments,
        WSConnectConfig {
            connect_portfolio_stream: true,
            portfolio_stream_update_types: Some(HashSet::from([
                PortfolioUpdateType::Order,
                PortfolioUpdateType::Position,
                PortfolioUpdateType::Holding,
            ])),
            portfolio_feed_callback: Some(Box::new(portfolio_feed_handler)),
            // Open TWO parallel market-data sockets so each carries its
            // own subscription mode. `options_chain_full` (slot 2)
            // stays on `Full` mode, `execution_zone_d30` (slot 1) uses
            // `full_d30`. See `examples/ws_multi` for the full 5-slot
            // fan-out pattern.
            market_data_streams: vec![
                WsChannelConfig {
                    connection: WsConnectionRole::OptionsChainFull.id(),
                    callback: Some(Box::new(options_chain_handler)),
                },
                WsChannelConfig {
                    connection: WsConnectionRole::ExecutionZoneD30.id(),
                    callback: Some(Box::new(exec_zone_handler)),
                },
            ],
        },
    )
    .await
    .unwrap();

    let api_client = api_client.lock().await;

    // Subscribe on slot 2 (OptionsChainFull) — indices piggy-back on
    // this slot just to produce visible traffic for the demo.
    api_client
        .send_market_data_feed_v3_message_by_role(
            WsConnectionRole::OptionsChainFull,
            MarketDataV3Call::SubscribeInstrument(MessageDataV3 {
                mode: ModeTypeV3::Full,
                instrument_keys: vec![
                    "NSE_INDEX|NIFTY LARGEMID250".to_string(),
                    "NSE_INDEX|Nifty Auto".to_string(),
                    "NSE_INDEX|Nifty Midcap 50".to_string(),
                ],
            }),
        )
        .await
        .unwrap();

    // Subscribe on slot 1 (ExecutionZoneD30) with a single `full_d30`
    // key so the example exercises the native 30-level depth path.
    api_client
        .send_market_data_feed_v3_message_by_role(
            WsConnectionRole::ExecutionZoneD30,
            MarketDataV3Call::SubscribeInstrument(MessageDataV3 {
                mode: ModeTypeV3::FullD30,
                instrument_keys: vec!["NSE_FO|63412".to_string()],
            }),
        )
        .await
        .unwrap();

    tokio::select! {
        _ = join_all(tasks_vec) => {}
        _ = signal::ctrl_c() => {}
    };
}
