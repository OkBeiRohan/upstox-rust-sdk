//! Upstox Plus 5-WS fan-out example.
//!
//! Opens one market-data WebSocket per `WsConnectionRole` (5 slots
//! total), plus the portfolio stream. Each slot's callback stamps the
//! role onto its log line so operators can see per-slot traffic
//! independently — crucial for verifying that `ConstituentsD30` and
//! `ExecutionZoneD30` land on separate physical sockets and both
//! materialise 30-level depth (Upstox Plus' 50-per-connection
//! `full_d30` cap applies per socket).
//!
//! Run with `cargo run --example ws_multi`. Requires a valid Upstox
//! access token (the SDK triggers OAuth if missing).

use {
    dotenvy::dotenv,
    futures::future::join_all,
    std::{collections::HashSet, env},
    tokio::signal,
    upstox_rust_sdk::{
        ALL_WS_CONNECTION_ROLES,
        client::{
            ApiClient, AutomateLoginConfig, LoginConfig, MailProvider, WSConnectConfig,
            WsChannelConfig,
        },
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

fn make_callback(
    role: WsConnectionRole,
) -> Box<dyn FnMut(MarketDataFeedV3Response) + Send + Sync> {
    Box::new(move |data: MarketDataFeedV3Response| {
        println!("[{}] {data:?}", role.name());
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let _ = dotenv();

    let api_key: String = env::var(UPLINK_API_KEY_ENV).unwrap();
    let (fetch_instruments, schedule_refresh_instruments) = (false, false);

    // Fan out one channel per role — 5 physical WS connections.
    let market_data_streams: Vec<WsChannelConfig> = ALL_WS_CONNECTION_ROLES
        .iter()
        .map(|role| WsChannelConfig {
            connection: role.id(),
            callback: Some(make_callback(*role)),
        })
        .collect();

    let portfolio_cb = Box::new(|data: PortfolioFeedResponse| {
        println!("[portfolio] {data:?}");
    });

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
            ])),
            portfolio_feed_callback: Some(portfolio_cb),
            market_data_streams,
        },
    )
    .await
    .unwrap();

    let api_client = api_client.lock().await;

    // Send a representative subscribe on each slot using its default mode.
    api_client
        .send_market_data_feed_v3_message_by_role(
            WsConnectionRole::IndicesLtpc,
            MarketDataV3Call::SubscribeInstrument(MessageDataV3 {
                mode: ModeTypeV3::LTPC,
                instrument_keys: vec![
                    "NSE_INDEX|Nifty 50".to_string(),
                    "NSE_INDEX|Nifty Bank".to_string(),
                    "NSE_INDEX|India VIX".to_string(),
                ],
            }),
        )
        .await
        .unwrap();

    api_client
        .send_market_data_feed_v3_message_by_role(
            WsConnectionRole::OptionsChainFull,
            MarketDataV3Call::SubscribeInstrument(MessageDataV3 {
                mode: ModeTypeV3::Full,
                instrument_keys: vec!["NSE_INDEX|NIFTY LARGEMID250".to_string()],
            }),
        )
        .await
        .unwrap();

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

    api_client
        .send_market_data_feed_v3_message_by_role(
            WsConnectionRole::ConstituentsD30,
            MarketDataV3Call::SubscribeInstrument(MessageDataV3 {
                mode: ModeTypeV3::FullD30,
                instrument_keys: vec!["NSE_EQ|INE040A01034".to_string()],
            }),
        )
        .await
        .unwrap();

    // ExpansionFull left idle — demonstrates reserving a slot for
    // run-time expansion (e.g. a second enabled universe).

    tokio::select! {
        _ = join_all(tasks_vec) => {}
        _ = signal::ctrl_c() => {}
    };
}
