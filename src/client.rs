//! ## Get started
//!
//! The code below represents an API client that connects websockets and uses handlers to handle incoming data.
//!
//!
//! ```rust
//! use {
//!     dotenvy::dotenv,
//!     futures::future::join_all,
//!     std::{collections::HashSet, env},
//!     tokio::signal,
//!     upstox_rust_sdk::{
//!         client::{ApiClient, AutomateLoginConfig, LoginConfig, MailProvider, WSConnectConfig},
//!         constants::UPLINK_API_KEY_ENV,
//!         models::ws::{
//!             market_data_feed_v3_message::{MessageDataV3, ModeTypeV3},
//!             portfolio_feed_request::PortfolioUpdateType,
//!             portfolio_feed_response::PortfolioFeedResponse,
//!         },
//!         protos::market_data_feed_v3::FeedResponse as MarketDataFeedV3Response,
//!         ws_client::MarketDataV3Call,
//!     },
//! };

//! #[tokio::main]
//! async fn main() {
//!     tracing_subscriber::fmt::init();
//!     let _ = dotenv();

//!     let portfolio_feed_handler = |data: PortfolioFeedResponse| {
//!         println!("{:?}", data);
//!     };
//!     let market_data_feed_v3_handler = |data: MarketDataFeedV3Response| {
//!         println!("{:?}", data);
//!     };

//!     let api_key: String = env::var(UPLINK_API_KEY_ENV).unwrap();
//!     let (fetch_instruments, schedule_refresh_instruments) = (false, false);

//!     // ApiClient with websockets connected and handler specified
//!     let (api_client, tasks_vec) = ApiClient::new(
//!         &api_key,
//!         LoginConfig {
//!             authorize: true,
//!             automate_login_config: Some(AutomateLoginConfig {
//!                 automate_login: true,
//!                 schedule_login: false,
//!                 automate_fetching_otp: true,
//!                 mail_provider: Some(MailProvider::Google),
//!             }),
//!         },
//!         fetch_instruments,
//!         schedule_refresh_instruments,
//!         // Configuration to connect and handle websocket data.
//!         WSConnectConfig {
//!             connect_portfolio_stream: true,
//!             connect_market_data_stream_v3: true,
//!             // Select which portfolio data to fetch
//!             portfolio_stream_update_types: Some(HashSet::from([
//!                 PortfolioUpdateType::Order,
//!                 PortfolioUpdateType::Position,
//!                 PortfolioUpdateType::Holding,
//!             ])),
//!             // Handle portfolio data feed
//!             portfolio_feed_callback: Some(Box::new(portfolio_feed_handler)),
//!             // Handle market data feed
//!             market_data_feed_v3_callback: Some(Box::new(market_data_feed_v3_handler)),
//!         },
//!     )
//!     .await
//!     .unwrap();

//!     let api_client = api_client.lock().await;
//!     api_client
//!         .send_market_data_feed_v3_message(MarketDataV3Call::SubscribeInstrument(MessageDataV3 {
//!             mode: ModeTypeV3::Full,
//!             instrument_keys: vec![
//!                 "NSE_INDEX|NIFTY LARGEMID250".to_string(),
//!                 "NSE_INDEX|Nifty Auto".to_string(),
//!                 "NSE_INDEX|Nifty Midcap 50".to_string(),
//!             ],
//!         }))
//!         .await
//!         .unwrap();

//!     // This ensures that app continues running until the websockets die if connected or until SIGINT occurs
//!     tokio::select! {
//!         _ = join_all(tasks_vec) => {}
//!         _ = signal::ctrl_c() => {}
//!     };
//! }

//!
//! ```
//!
//! The code below represents an API client that logs user in, allowing usage of authorised endpoints, automates the login process including fetching OTP via GMail and scheduling it daily.
//!
//!
//! ```rust
//!
//! #[tokio::main]
//! async fn main() {
//!     tracing_subscriber::fmt::init();
//!     let _ = dotenv();

//!     let api_key: String = env::var(UPLINK_API_KEY_ENV).unwrap();
//!     let (fetch_instruments, schedule_refresh_instruments) = (false, false);

//!     // ApiClient which logs in automatically and schedules relogin daily when token expires
//!     let (_api_client, tasks_vec) = ApiClient::new(
//!         &api_key,
//!         LoginConfig {
//!             authorize: true,
//!             automate_login_config: Some(AutomateLoginConfig {
//!                 // geckodriver or chromedriver binary must be running locally with port specified in env to use automatic login or schedule login.
//!                 // ./geckodriver --binary "~/.local/share/flatpak/exports/bin/org.mozilla.firefox" --profile-root "~/.var/app/org.mozilla.firefox/cache/mozilla/firefox/cv70hco5.default-release"

//!                 // Either GOOGLE_AUTHORIZATION_CODE environment must not be set or must be recent if using for the first time or refresh_token.txt has been deleted
//!                 automate_login: true,
//!                 // Relogin is scheduled at 3:30 AM IST daily.
//!                 schedule_login: true,
//!                 // Fetch OTP automatically from inbox by providing email IMAP access using env variables. For GMail, login page will be opened in your default web browsers and permission must be granted.
//!                 automate_fetching_otp: true,
//!                 mail_provider: Some(MailProvider::Google),
//!             }),
//!         },
//!         fetch_instruments,
//!         schedule_refresh_instruments,
//!         WSConnectConfig {
//!             connect_portfolio_stream: false,
//!             connect_market_data_stream_v3: false,
//!             portfolio_stream_update_types: None,
//!             portfolio_feed_callback: None::<Box<dyn FnMut(PortfolioFeedResponse) + Send + Sync>>,
//!             market_data_feed_v3_callback: None::<
//!                 Box<dyn FnMut(MarketDataFeedV3Response) + Send + Sync>,
//!             >,
//!         },
//!     )
//!     .await
//!     .unwrap();

//!     // This ensures that app continues running until SIGINT occurs
//!     tokio::select! {
//!         _ = join_all(tasks_vec) => {}
//!         _ = signal::ctrl_c() => {}
//!     };
//! }
//!
//!
//! ```
//!
//! The code below represents an API client that fetches the complete list of instruments and stores it in the API Client. It also schedules daily refresh of the list.
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     tracing_subscriber::fmt::init();
//!     let _ = dotenv();

//!     let api_key: String = env::var(UPLINK_API_KEY_ENV).unwrap();
//!     let (fetch_instruments, schedule_refresh_instruments) = (true, true);

//!     // ApiClient which fetches instruments, schedules instruments refresh daily and stores it in ApiClient
//!     let (api_client, tasks_vec) = ApiClient::new(
//!         &api_key,
//!         LoginConfig {
//!             authorize: false,
//!             automate_login_config: Some(AutomateLoginConfig {
//!                 automate_login: false,
//!                 schedule_login: false,
//!                 automate_fetching_otp: false,
//!                 mail_provider: Some(MailProvider::Google),
//!             }),
//!         },
//!         // Fetch all instruments data from UPSTOX and store it in the ApiClient.
//!         fetch_instruments,
//!         // Refresh instruments data daily at 6:30 AM.
//!         schedule_refresh_instruments,
//!         WSConnectConfig {
//!             connect_portfolio_stream: false,
//!             connect_market_data_stream_v3: false,
//!             portfolio_stream_update_types: None,
//!             portfolio_feed_callback: None::<Box<dyn FnMut(PortfolioFeedResponse) + Send + Sync>>,
//!             market_data_feed_v3_callback: None::<
//!                 Box<dyn FnMut(MarketDataFeedV3Response) + Send + Sync>,
//!             >,
//!         },
//!     )
//!     .await
//!     .unwrap();

//!     {
//!         // Ensure that api_client mutex guard goes out of scope when no longer needed.
//!         let api_client: MutexGuard<ApiClient> = api_client.lock().await;
//!         print!(
//!             "{:?}",
//!             api_client
//!                 .instruments
//!                 .as_ref()
//!                 .unwrap()
//!                 .get(&ExchangeSegment::NseIndex)
//!                 .unwrap()
//!                 .get("INDEX")
//!                 .unwrap()
//!         );
//!         std::io::stdout().flush().unwrap();
//!     }

//!     // This ensures that app continues running until SIGINT occurs
//!     tokio::select! {
//!         _ = join_all(tasks_vec) => {}
//!         _ = signal::ctrl_c() => {}
//!     };
//! }
//! ```
//!
//! The code below represents the basic usage of API Client to fetch data via REST APIs.
//!
//! ```rust
//! use {
//!     dotenvy::dotenv,
//!     futures::future::join_all,
//!     std::env,
//!     tokio::{
//!         signal,
//!         sync::MutexGuard,
//!         time::{Instant, sleep},
//!     },
//!     tracing::info,
//!     upstox_rust_sdk::{
//!         client::{ApiClient, AutomateLoginConfig, LoginConfig, MailProvider, WSConnectConfig},
//!         constants::UPLINK_API_KEY_ENV,
//!         models::{
//!             ProductType, TransactionType,
//!             charges::brokerage_details_request::BrokerageDetailsRequest,
//!             success_response::SuccessResponse,
//!             user::{
//!                 fund_and_margin_request::SegmentType,
//!                 fund_and_margin_response::FundAndMarginResponse, profile_response::ProfileResponse,
//!             },
//!             ws::portfolio_feed_response::PortfolioFeedResponse,
//!         },
//!         protos::market_data_feed_v3::FeedResponse as MarketDataFeedV3Response,
//!         rate_limiter::RateLimitExceeded,
//!     },
//! };

//! #[tokio::main]
//! async fn main() {
//!     tracing_subscriber::fmt::init();
//!     let _ = dotenv();

//!     let api_key: String = env::var(UPLINK_API_KEY_ENV).unwrap();
//!     let (fetch_instruments, schedule_refresh_instruments) = (true, false);

//!     let (api_client, tasks_vec) = ApiClient::new(
//!         &api_key,
//!         LoginConfig {
//!             authorize: true,
//!             automate_login_config: Some(AutomateLoginConfig {
//!                 automate_login: true,
//!                 schedule_login: false,
//!                 automate_fetching_otp: true,
//!                 mail_provider: Some(MailProvider::Google),
//!             }),
//!         },
//!         fetch_instruments,
//!         schedule_refresh_instruments,
//!         WSConnectConfig {
//!             connect_portfolio_stream: false,
//!             connect_market_data_stream_v3: false,
//!             portfolio_stream_update_types: None,
//!             portfolio_feed_callback: None::<Box<dyn FnMut(PortfolioFeedResponse) + Send + Sync>>,
//!             market_data_feed_v3_callback: None::<
//!                 Box<dyn FnMut(MarketDataFeedV3Response) + Send + Sync>,
//!             >,
//!         },
//!     )
//!     .await
//!     .unwrap();

//!     {
//!         let api_client: MutexGuard<ApiClient> = api_client.lock().await;

//!         // User Endpoints
//!         let _profile: SuccessResponse<ProfileResponse> =
//!             api_client.get_profile().await.unwrap().unwrap();
//!         info!("Profile: {:?}", _profile);

//!         // Charges Endpoints
//!         let _charges_result = api_client
//!             .get_brokerage_details(BrokerageDetailsRequest {
//!                 instrument_token: "NSE_EQ|INE806T01012".to_string(),
//!                 quantity: 2,
//!                 product: ProductType::I,
//!                 transaction_type: TransactionType::Buy,
//!                 price: 1575.00,
//!             })
//!             .await;

//!         if let Err(e) = _charges_result {
//!             let next_allowed_at = match e {
//!                 RateLimitExceeded::PerSecond { next_allowed_at } => next_allowed_at,
//!                 RateLimitExceeded::PerMinute { next_allowed_at } => next_allowed_at,
//!                 RateLimitExceeded::PerThirtyMinutes { next_allowed_at } => next_allowed_at,
//!             };
//!             info!(
//!                 "Rate limit exceeded. Sleeping for {:?}",
//!                 next_allowed_at - Instant::now()
//!             );
//!             sleep(next_allowed_at - Instant::now()).await;
//!         }

//!         let charges = api_client
//!             .get_brokerage_details(BrokerageDetailsRequest {
//!                 instrument_token: "NSE_EQ|INE806T01012".to_string(),
//!                 quantity: 2,
//!                 product: ProductType::I,
//!                 transaction_type: TransactionType::Buy,
//!                 price: 1575.00,
//!             })
//!             .await
//!             .unwrap()
//!             .unwrap();

//!         info!("Charges: {:?}", charges);

//!         let _funds_and_margin: SuccessResponse<FundAndMarginResponse> = api_client
//!             .get_fund_and_margin(Some(SegmentType::Sec))
//!             .await
//!             .unwrap()
//!             .unwrap();

//!         // This is just for usage illustration. All endpoints in https://upstox.com/developer/api-documentation/open-api are available via the ApiClient.
//!     }

//!     // This ensures that app continues running until SIGINT occurs
//!     tokio::select! {
//!         _ = join_all(tasks_vec) => {}
//!         _ = signal::ctrl_c() => {}
//!     };
//! }
//!
//! ```

use {
    crate::{
        constants::{APIVersion, BaseUrlType},
        models::{
            ExchangeSegment,
            error_response::ErrorResponse,
            instruments::instruments_response::InstrumentsResponse,
            success_response::SuccessResponse,
            user::profile_response::ProfileResponse,
            ws::{
                portfolio_feed_request::PortfolioUpdateType,
                portfolio_feed_response::PortfolioFeedResponse,
            },
        },
        rate_limiter::{ApiRateLimiter, RateLimitExceeded, RateLimitProfile},
        utils::create_url,
        ws_client::{
            MAX_MARKET_DATA_CONNECTIONS, MarketDataFeedV3CallbackBox, MarketDataFeedV3ClientPool,
            PortfolioFeedClient, WsConnectionId,
        },
    },
    chrono::FixedOffset,
    ezsockets::Client as EzClient,
    reqwest::{Client as ReqwestClient, Method, RequestBuilder, Response},
    serde::Serialize,
    std::{
        collections::{HashMap, HashSet},
        sync::Arc,
        time::{Duration, Instant},
    },
    tokio::{
        sync::{Mutex, MutexGuard},
        task::JoinHandle,
    },
    tokio_cron_scheduler::{Job, JobScheduler},
    tracing::info,
};

/// Account-level capability flags the SDK enforces at the feature
/// boundary. Forwarded to the `ApiClient` at construction time via
/// [`ApiClient::new_with_capabilities`].
///
/// Defaults (`Default::default()`) model a **non-Plus, non-SEBI**
/// account: 2 market-data WebSocket connections, no `full_d30`, no
/// expired-instruments endpoints, 10 orders/second peak. Flip the
/// flags **only** when the underlying Upstox account actually has the
/// matching entitlement — every flag changes the SDK's contract with
/// the broker and a mis-configuration either wastes rate-limit budget
/// or triggers silent exchange rejections.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClientCapabilities {
    /// `true` when the Upstox account is subscribed to
    /// [Upstox Plus](https://upstox.com/developer/api-documentation/announcements/websocket-plus).
    /// Plus unlocks:
    ///
    /// - Up to [`MAX_MARKET_DATA_CONNECTIONS`] = 5 market-data
    ///   WebSocket connections (standard: `MAX_MARKET_DATA_CONNECTIONS_STANDARD` = 2).
    /// - [`ModeTypeV3::FullD30`][crate::models::ws::market_data_feed_v3_message::ModeTypeV3::FullD30]
    ///   subscriptions (30-level market depth).
    /// - Expired-instruments REST endpoints (`get_expiries`,
    ///   `get_expired_option_contracts`,
    ///   `get_expired_future_contracts`,
    ///   `get_expired_historical_candle_data`).
    ///
    /// Default `false` — Plus-only calls are rejected with
    /// [`RateLimitExceeded::FeatureRequiresPlus`].
    pub is_plus_user: bool,

    /// `true` when the user's Upstox app is SEBI-registered as an
    /// algo strategy. Only affects the per-second cap on the
    /// order-placement rate-limit bucket (50/s registered vs 10/s
    /// non-registered, per the
    /// [rate-limiting docs](https://upstox.com/developer/api-documentation/rate-limiting)).
    ///
    /// Default `false` — the conservative 10/s cap applies.
    pub is_sebi_registered: bool,
}

impl ClientCapabilities {
    /// Short-hand for `ClientCapabilities { is_plus_user: false, is_sebi_registered: false }`.
    pub const fn standard() -> Self {
        Self {
            is_plus_user: false,
            is_sebi_registered: false,
        }
    }

    /// Short-hand for a Plus + SEBI-registered account.
    pub const fn plus_sebi() -> Self {
        Self {
            is_plus_user: true,
            is_sebi_registered: true,
        }
    }

    /// Which [`RateLimitProfile`] the rate limiter should run under.
    pub const fn rate_limit_profile(self) -> RateLimitProfile {
        if self.is_sebi_registered {
            RateLimitProfile::SebiRegistered
        } else {
            RateLimitProfile::RegularAlgo
        }
    }

    /// Max simultaneous market-data WebSockets the broker allows for
    /// this capability set. Used by
    /// [`ApiClient::connect_market_data_feed_v3`] to short-circuit a
    /// bad request before it reaches Upstox.
    pub const fn max_market_data_connections(self) -> usize {
        if self.is_plus_user {
            MAX_MARKET_DATA_CONNECTIONS
        } else {
            crate::ws_client::MAX_MARKET_DATA_CONNECTIONS_STANDARD
        }
    }
}

pub struct ApiClient {
    pub(crate) client: ReqwestClient,
    pub(crate) api_key: String,
    pub(crate) token: Option<String>,
    /// Optional value for the `X-Algo-Name` header (2026-04-01 SEBI
    /// circular). Injected on order-placement-bucket endpoints only;
    /// leave `None` unless the Upstox app has registered algo strategies.
    pub(crate) algo_name: Option<String>,
    /// Account-level capability flags. Populated at construction and
    /// never mutated afterwards (flipping `is_plus_user` mid-session
    /// would silently drop the extra WS slots + `full_d30` subs).
    pub(crate) capabilities: ClientCapabilities,
    pub instruments: Option<HashMap<ExchangeSegment, HashMap<String, Vec<InstrumentsResponse>>>>,
    pub portfolio_feed_client:
        Option<EzClient<PortfolioFeedClient<Box<dyn FnMut(PortfolioFeedResponse) + Send + Sync>>>>,
    /// Pool of up to [`MAX_MARKET_DATA_CONNECTIONS`] parallel market-data
    /// WebSockets (Upstox Plus allows 5 per user, standard accounts 2).
    /// Each slot is independent — typical high-frequency clients fan
    /// out one role per slot (constituents_d30, execution_zone_d30,
    /// options_chain_full, indices_ltpc, expansion_full) and
    /// subscribe against each directly. See [`WsConnectionRole`] for
    /// the canonical role → id mapping.
    pub market_data_feed_v3_clients: MarketDataFeedV3ClientPool,
    pub rate_limiter: ApiRateLimiter,
}

impl ApiClient {
    /// Construct an `ApiClient` with [`ClientCapabilities::default`]
    /// — non-Plus, non-SEBI. Plus-only APIs return
    /// [`RateLimitExceeded::FeatureRequiresPlus`]; the order-bucket
    /// rate limiter caps per-second traffic at 10 req/s.
    ///
    /// Use [`ApiClient::new_with_capabilities`] to unlock Plus or
    /// SEBI-registered behaviour. Chain [`ApiClient::set_algo_name`]
    /// after construction when the `X-Algo-Name` header is required.
    pub async fn new(
        api_key: &str,
        login_config: LoginConfig,
        fetch_instruments: bool,
        schedule_refresh_instruments: bool,
        ws_connect_config: WSConnectConfig,
    ) -> Result<(Arc<Mutex<ApiClient>>, Vec<JoinHandle<()>>), String> {
        Self::new_with_capabilities(
            api_key,
            login_config,
            fetch_instruments,
            schedule_refresh_instruments,
            ws_connect_config,
            ClientCapabilities::default(),
        )
        .await
    }

    /// Construct an `ApiClient` with a specific [`RateLimitProfile`].
    ///
    /// Kept as a thin compat wrapper — it leaves `is_plus_user = false`
    /// so consumers that want Plus-only features must migrate to
    /// [`ApiClient::new_with_capabilities`]. Scheduled for removal in
    /// v3.
    #[deprecated(
        note = "Use `new_with_capabilities(ClientCapabilities)` — the profile-only \
                constructor cannot express `is_plus_user` and leaves Plus-only \
                APIs refusing with `RateLimitExceeded::FeatureRequiresPlus`."
    )]
    pub async fn new_with_profile(
        api_key: &str,
        login_config: LoginConfig,
        fetch_instruments: bool,
        schedule_refresh_instruments: bool,
        ws_connect_config: WSConnectConfig,
        rate_limit_profile: RateLimitProfile,
    ) -> Result<(Arc<Mutex<ApiClient>>, Vec<JoinHandle<()>>), String> {
        let capabilities = ClientCapabilities {
            is_plus_user: false,
            is_sebi_registered: matches!(rate_limit_profile, RateLimitProfile::SebiRegistered),
        };
        Self::new_with_capabilities(
            api_key,
            login_config,
            fetch_instruments,
            schedule_refresh_instruments,
            ws_connect_config,
            capabilities,
        )
        .await
    }

    /// Construct an `ApiClient` with an explicit
    /// [`ClientCapabilities`] set. Preferred over
    /// [`ApiClient::new`] whenever the account is on Upstox Plus
    /// or has a SEBI-registered algo.
    pub async fn new_with_capabilities(
        api_key: &str,
        login_config: LoginConfig,
        fetch_instruments: bool,
        schedule_refresh_instruments: bool,
        ws_connect_config: WSConnectConfig,
        capabilities: ClientCapabilities,
    ) -> Result<(Arc<Mutex<ApiClient>>, Vec<JoinHandle<()>>), String> {
        let api_client = ApiClient {
            client: ReqwestClient::new(),
            api_key: api_key.to_string(),
            token: None,
            algo_name: None,
            capabilities,
            instruments: None,
            portfolio_feed_client: None,
            // `[const { None }; N]` is stable on Rust 2024; keeps the
            // pool empty until `connect_market_data_feed_v3` populates
            // individual slots.
            market_data_feed_v3_clients: [const { None }; MAX_MARKET_DATA_CONNECTIONS],
            rate_limiter: ApiRateLimiter::new(capabilities.rate_limit_profile()),
        };

        let shared_api_client = Arc::new(Mutex::new(api_client));
        let mut tasks_vec = Vec::<JoinHandle<()>>::new();

        let scheduler = JobScheduler::new().await.unwrap();
        scheduler.start().await.unwrap();
        scheduler.shutdown_on_ctrl_c();

        if fetch_instruments {
            let mut api_client = shared_api_client.lock().await;
            api_client.instruments =
                Some(Self::parse_instruments(api_client.get_instruments().await?));
            if schedule_refresh_instruments {
                Self::schedule_refresh_instruments(&scheduler, &shared_api_client).await;
            }
        }

        if !login_config.authorize {
            return Ok((shared_api_client, tasks_vec));
        }

        {
            let mut api_client = shared_api_client.lock().await;
            api_client.login(&login_config).await?;

            if ws_connect_config.connect_portfolio_stream {
                let portfolio_feed_task = api_client
                    .connect_portfolio_feed(
                        ws_connect_config.portfolio_stream_update_types,
                        ws_connect_config.portfolio_feed_callback,
                    )
                    .await?;
                tasks_vec.push(portfolio_feed_task);
            }

            // One WS per `WsChannelConfig` entry. Connections are
            // STAGGERED — we wait for each slot's WS handshake to
            // finish (or time out) before issuing the next connect.
            //
            // Why stagger: the 2026-05-12 NOB session showed Upstox
            // throttles a single account that opens 3+ WSes in rapid
            // succession (<100 ms apart). Symptoms: only 1 of 5 slot
            // handshakes completes; the other 4 actor tasks loop on
            // a single-use auth URL that the broker has already
            // rejected, never recovering until the supervisor
            // (`reconnect_market_data_by_id`) drops the dead pool
            // entry and fetches a fresh URL. Confirmed by community
            // threads — Plus users report consistent failures past
            // 3 simultaneous connections even though the documented
            // cap is 5.
            //
            // Sequential boot trades a small extra latency
            // (~`PER_SLOT_OPEN_BUDGET_MS` × N) for a much higher
            // success rate on the first try. A slot that doesn't
            // open within the budget is left as a "best-effort"
            // task — its actor task keeps the slot's `is_open`
            // flag at false; the supervisor's watchdog will drive
            // a reconnect with a fresh URL when it next polls.
            //
            // Reject duplicate slot ids up front — two callbacks for
            // the same pool id would lose the first callback silently
            // (the second call would error with "slot already
            // connected") or race depending on ordering.
            //
            // Per-slot wait budget: long enough for a cold CloudFront
            // PoP TLS+WS handshake (~1-3 s) plus a brief Upstox-side
            // accept window. Bigger budgets push the whole 5-slot
            // boot past `PreMarket → MarketOpen` if Upstox is fully
            // throttled — the watchdog handles those slots in the
            // background instead.
            const PER_SLOT_OPEN_BUDGET: Duration = Duration::from_secs(8);
            const POLL_INTERVAL: Duration = Duration::from_millis(100);
            // Spacing between connect calls (skipped on the first).
            // Gives Upstox's anti-abuse layer a clear "this is one
            // user opening sequential connections, not a flood" signal.
            const INTER_CONNECT_DELAY: Duration = Duration::from_millis(750);

            let mut seen_slots: Vec<usize> =
                Vec::with_capacity(ws_connect_config.market_data_streams.len());
            let total_slots = ws_connect_config.market_data_streams.len();
            for (idx, channel) in
                ws_connect_config.market_data_streams.into_iter().enumerate()
            {
                if !channel.connection.is_valid() {
                    return Err(format!(
                        "WsChannelConfig slot {} out of range (max {MAX_MARKET_DATA_CONNECTIONS})",
                        channel.connection.0
                    ));
                }
                if seen_slots.contains(&channel.connection.0) {
                    return Err(format!(
                        "duplicate WsChannelConfig for slot {}",
                        channel.connection.0
                    ));
                }
                seen_slots.push(channel.connection.0);

                if idx > 0 {
                    tokio::time::sleep(INTER_CONNECT_DELAY).await;
                }

                let slot_id = channel.connection.0;
                let task = api_client
                    .connect_market_data_feed_v3(channel.connection, channel.callback)
                    .await?;
                tasks_vec.push(task);
                // Borrow the just-pushed task so the wait loop can
                // bail out early if the actor exits (which happens
                // immediately under `max_initial_connect_attempts(1)`
                // when Upstox rejects the handshake — no point
                // burning the rest of the per-slot budget polling
                // an `is_open` flag the dead actor will never flip).
                let task_handle = tasks_vec
                    .last()
                    .expect("just pushed; tasks_vec is non-empty");

                let wait_start = Instant::now();
                let mut opened = false;
                let mut actor_died = false;
                while wait_start.elapsed() < PER_SLOT_OPEN_BUDGET {
                    if api_client.is_market_data_connected(channel.connection) {
                        opened = true;
                        break;
                    }
                    if task_handle.is_finished() {
                        actor_died = true;
                        break;
                    }
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
                if opened {
                    info!(
                        slot = slot_id,
                        index = idx + 1,
                        total = total_slots,
                        elapsed_ms = wait_start.elapsed().as_millis() as u64,
                        "market-data WS slot opened during boot stagger"
                    );
                } else if actor_died {
                    tracing::warn!(
                        slot = slot_id,
                        index = idx + 1,
                        total = total_slots,
                        elapsed_ms = wait_start.elapsed().as_millis() as u64,
                        "market-data WS slot actor exited before handshake \
                         completed (Upstox likely throttled this connection) \
                         — supervisor watchdog will reconnect with a fresh \
                         auth URL on its next cycle"
                    );
                } else {
                    tracing::warn!(
                        slot = slot_id,
                        index = idx + 1,
                        total = total_slots,
                        budget_ms = PER_SLOT_OPEN_BUDGET.as_millis() as u64,
                        "market-data WS slot did NOT open within boot-stagger \
                         budget (actor still running; handshake in progress \
                         or stuck) — supervisor watchdog will check later"
                    );
                }
            }
        }

        if let Some(automate_login_config) = login_config.automate_login_config {
            if automate_login_config.schedule_login {
                Self::schedule_auto_login(&scheduler, &shared_api_client, login_config).await;
            }
        }
        Ok((shared_api_client, tasks_vec))
    }

    pub(crate) async fn get(
        &self,
        endpoint: &str,
        authorized: bool,
        params: Option<&Vec<(String, String)>>,
        base_url_type: BaseUrlType,
        api_version: APIVersion,
    ) -> Result<Response, RateLimitExceeded> {
        self.request::<()>(
            Method::GET,
            endpoint,
            authorized,
            params,
            None,
            None,
            base_url_type,
            api_version,
        )
        .await
    }

    pub(crate) async fn post<T>(
        &self,
        endpoint: &str,
        authorized: bool,
        json_body: Option<&T>,
        form_body: Option<&Vec<(String, String)>>,
        base_url_type: BaseUrlType,
        api_version: APIVersion,
    ) -> Result<Response, RateLimitExceeded>
    where
        T: Serialize + ?Sized,
    {
        self.request(
            Method::POST,
            endpoint,
            authorized,
            None,
            json_body,
            form_body,
            base_url_type,
            api_version,
        )
        .await
    }

    pub(crate) async fn put<T>(
        &self,
        endpoint: &str,
        authorized: bool,
        json_body: Option<&T>,
        form_body: Option<&Vec<(String, String)>>,
        base_url_type: BaseUrlType,
        api_version: APIVersion,
    ) -> Result<Response, RateLimitExceeded>
    where
        T: Serialize + ?Sized,
    {
        self.request(
            Method::PUT,
            endpoint,
            authorized,
            None,
            json_body,
            form_body,
            base_url_type,
            api_version,
        )
        .await
    }

    pub(crate) async fn delete<T>(
        &self,
        endpoint: &str,
        authorized: bool,
        params: Option<&Vec<(String, String)>>,
        json_body: Option<&T>,
        base_url_type: BaseUrlType,
        api_version: APIVersion,
    ) -> Result<Response, RateLimitExceeded>
    where
        T: Serialize + ?Sized,
    {
        self.request::<T>(
            Method::DELETE,
            endpoint,
            authorized,
            params,
            json_body,
            None,
            base_url_type,
            api_version,
        )
        .await
    }

    async fn request<T>(
        &self,
        method: Method,
        endpoint: &str,
        authorized: bool,
        params: Option<&Vec<(String, String)>>,
        json_body: Option<&T>,
        form_body: Option<&Vec<(String, String)>>,
        base_url_type: BaseUrlType,
        api_version: APIVersion,
    ) -> Result<Response, RateLimitExceeded>
    where
        T: Serialize + ?Sized,
    {
        // Self-pacing: block until a slot is free in the appropriate
        // bucket. Pre-v3 SDK released here with a non-blocking
        // `check_rate_limit` that surfaced `RateLimitExceeded::Per*`
        // up to the caller, forcing every consumer to wrap each call
        // in a retry-with-sleep loop. v3 (this file) flips to
        // self-pacing — `acquire_slot` polls the bucket + sleeps
        // until a slot is available, so consumers only see
        // `RateLimitExceeded` if Upstox's account-wide cap (which
        // can be exhausted by a prior process) trips at the
        // network layer.
        self.rate_limiter.acquire_slot(endpoint).await;
        let url: String = create_url(base_url_type, api_version, endpoint);

        if authorized && !self.token.is_some() {
            panic!(
                "{}",
                format!(
                    "Cannot make authorized requests as client is not authorized: {}",
                    url
                )
            );
        }

        let mut request: RequestBuilder = match method.clone() {
            Method::GET => self.client.get(url),
            Method::POST => self.client.post(url),
            Method::PUT => self.client.put(url),
            Method::DELETE => self.client.delete(url),
            other => return Err(RateLimitExceeded::UnsupportedMethod(other.to_string())),
        };

        if let Some(req_params) = params {
            request = request.query(req_params);
        }

        if let Some(req_json_body) = json_body {
            request = request.json(req_json_body);
        }

        if let Some(req_form_body) = form_body {
            request = request.form(req_form_body);
        }

        if authorized {
            request = request.bearer_auth(&self.token.as_ref().unwrap());
        }
        request = request.header("Accept", "application/json");

        // 2026-04-01 SEBI/Exchange circular: apps with registered algo
        // strategies must stamp `X-Algo-Name` on every order-placement
        // request. We scope the header to the order-placement bucket
        // via `classify_endpoint` so standard reads (quotes/holidays/etc)
        // don't carry a header Upstox does not accept there.
        if let Some(ref name) = self.algo_name {
            if matches!(
                crate::rate_limiter::classify_endpoint(endpoint),
                crate::rate_limiter::RateLimitBucket::OrderPlacement
            ) {
                request = request.header("X-Algo-Name", name.as_str());
            }
        }

        // Transport errors (timeout, DNS, TLS, connection reset) used to
        // abort the process via `.unwrap()`. Surface them as a
        // `RateLimitExceeded::Network` variant so callers get a normal
        // `Err` and can retry with their own backoff.
        request
            .send()
            .await
            .map_err(|e| RateLimitExceeded::Network(e.to_string()))
    }

    /// Set the `X-Algo-Name` header value sent on every order-placement
    /// request. Pass `None` to clear. The header is never sent on
    /// standard-bucket endpoints (quotes, candles, user, market info,
    /// …). Required from 2026-04-01 for apps registered with an
    /// exchange-approved algo strategy; optional otherwise.
    pub fn set_algo_name(&mut self, algo_name: Option<String>) {
        self.algo_name = algo_name;
    }

    /// Returns the currently-configured `X-Algo-Name` value (for audit /
    /// logging). `None` when the header is not being sent.
    pub fn algo_name(&self) -> Option<&str> {
        self.algo_name.as_deref()
    }

    /// Immutable snapshot of the capability flags set at
    /// [`ApiClient::new_with_capabilities`] time. Useful for feature
    /// gates that want to present / hide UI before issuing a call.
    pub fn capabilities(&self) -> ClientCapabilities {
        self.capabilities
    }

    /// Refuse a Plus-only request up-front when the account does not
    /// carry the entitlement. Keeps the per-call guard clause down to
    /// a single `?` while producing a typed error the caller can match
    /// on. `feature` is used verbatim in the error payload for
    /// operator logging.
    pub(crate) fn ensure_plus_user(&self, feature: &str) -> Result<(), RateLimitExceeded> {
        if self.capabilities.is_plus_user {
            return Ok(());
        }
        Err(RateLimitExceeded::FeatureRequiresPlus(format!(
            "{feature} requires Upstox Plus (set ClientCapabilities::is_plus_user = true)"
        )))
    }

    pub(crate) async fn verify_authorization(&mut self) -> bool {
        let verify_response: Result<SuccessResponse<ProfileResponse>, ErrorResponse> =
            self.get_profile().await.unwrap();
        verify_response.map_or_else(
            |_| {
                info!("Upstox saved access token invalid");
                false
            },
            |_| {
                info!("Using valid access token from file");
                true
            },
        )
    }

    async fn schedule_refresh_instruments(
        scheduler: &JobScheduler,
        shared_api_client: &Arc<Mutex<ApiClient>>,
    ) {
        let shared_api_client_clone: Arc<Mutex<ApiClient>> = Arc::clone(&shared_api_client);
        let job: Job = Job::new_async_tz(
            "0 30 06 * * *",
            FixedOffset::east_opt(19800).unwrap(),
            move |_, _| {
                let api_client: Arc<Mutex<ApiClient>> = Arc::clone(&shared_api_client_clone);
                Box::pin(async move {
                    let mut client: MutexGuard<ApiClient> = api_client.lock().await;
                    if let Ok(instruments) = client.get_instruments().await {
                        client.instruments = Some(Self::parse_instruments(instruments));
                    }
                })
            },
        )
        .unwrap();

        scheduler.add(job).await.unwrap();
    }

    async fn schedule_auto_login(
        scheduler: &JobScheduler,
        shared_api_client: &Arc<Mutex<ApiClient>>,
        login_config: LoginConfig,
    ) {
        let shared_api_client_clone: Arc<Mutex<ApiClient>> = Arc::clone(&shared_api_client);
        let job: Job = Job::new_async_tz(
            "0 30 03 * * *",
            FixedOffset::east_opt(19800).unwrap(),
            move |_, _| {
                let api_client: Arc<Mutex<ApiClient>> = Arc::clone(&shared_api_client_clone);
                let login_config: LoginConfig = login_config.clone();
                Box::pin(async move {
                    let mut client: MutexGuard<ApiClient> = api_client.lock().await;
                    client.login(&login_config).await.unwrap();
                })
            },
        )
        .unwrap();

        scheduler.add(job).await.unwrap();
    }
}

#[derive(Clone)]
pub struct LoginConfig {
    pub authorize: bool,
    pub automate_login_config: Option<AutomateLoginConfig>,
}

#[derive(Clone, Copy)]
pub struct AutomateLoginConfig {
    pub automate_login: bool,
    pub schedule_login: bool, // At 3:30 AM IST daily
    pub automate_fetching_otp: bool,
    pub mail_provider: Option<MailProvider>,
}

#[derive(Clone, Copy)]
pub enum MailProvider {
    Google,
}

/// Configuration for one market-data WebSocket channel inside
/// [`WSConnectConfig::market_data_streams`].
///
/// Use [`ws_client::WsConnectionRole::id`][crate::ws_client::WsConnectionRole::id]
/// for `connection` when the slot matches one of the five named
/// roles; otherwise supply a raw [`WsConnectionId`] (`0..5`).
pub struct WsChannelConfig {
    /// Pool index to open (0..`MAX_MARKET_DATA_CONNECTIONS`).
    pub connection: WsConnectionId,
    /// Callback invoked for every decoded protobuf `FeedResponse`.
    /// `None` is legal for operators who only need the raw feed on the
    /// slot (debugging) but typical usage routes every tick.
    pub callback: Option<MarketDataFeedV3CallbackBox>,
}

pub struct WSConnectConfig {
    pub connect_portfolio_stream: bool,
    pub portfolio_stream_update_types: Option<HashSet<PortfolioUpdateType>>,
    pub portfolio_feed_callback: Option<Box<dyn FnMut(PortfolioFeedResponse) + Send + Sync>>,
    /// Up to [`MAX_MARKET_DATA_CONNECTIONS`] market-data WS channels,
    /// each on its own pool slot. An empty vec means "no market-data
    /// WS connections at startup" — the client can still open slots
    /// lazily via [`ApiClient::connect_market_data_feed_v3`] later.
    pub market_data_streams: Vec<WsChannelConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_capabilities_are_non_plus_non_sebi() {
        let c = ClientCapabilities::default();
        assert!(!c.is_plus_user);
        assert!(!c.is_sebi_registered);
        assert_eq!(c.rate_limit_profile(), RateLimitProfile::RegularAlgo);
        assert_eq!(
            c.max_market_data_connections(),
            crate::ws_client::MAX_MARKET_DATA_CONNECTIONS_STANDARD,
        );
    }

    #[test]
    fn sebi_registered_flips_rate_limit_profile() {
        let c = ClientCapabilities {
            is_plus_user: false,
            is_sebi_registered: true,
        };
        assert_eq!(c.rate_limit_profile(), RateLimitProfile::SebiRegistered);
        // Plus flag is still false → standard WS cap.
        assert_eq!(
            c.max_market_data_connections(),
            crate::ws_client::MAX_MARKET_DATA_CONNECTIONS_STANDARD,
        );
    }

    #[test]
    fn plus_user_flips_ws_connection_cap() {
        let c = ClientCapabilities {
            is_plus_user: true,
            is_sebi_registered: false,
        };
        assert_eq!(c.max_market_data_connections(), MAX_MARKET_DATA_CONNECTIONS);
        assert_eq!(c.rate_limit_profile(), RateLimitProfile::RegularAlgo);
    }

    #[test]
    fn plus_sebi_short_hand_lights_every_flag() {
        let c = ClientCapabilities::plus_sebi();
        assert!(c.is_plus_user);
        assert!(c.is_sebi_registered);
    }

    #[test]
    fn standard_short_hand_leaves_every_flag_false() {
        let c = ClientCapabilities::standard();
        assert!(!c.is_plus_user);
        assert!(!c.is_sebi_registered);
    }
}
