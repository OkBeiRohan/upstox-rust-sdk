use {
    crate::{
        client::ApiClient,
        constants::{
            APIVersion, BaseUrlType, WS_MARKET_DATA_FEED_AUTHORIZE_ENDPOINT,
            WS_PORTFOLIO_FEED_AUTHORIZE_ENDPOINT,
        },
        models::{
            error_response::ErrorResponse,
            success_response::SuccessResponse,
            ws::{
                AuthorizeFeedResponse,
                market_data_feed_v3_message::{
                    MarketDataFeedV3Message, MessageDataV3, MethodTypeV3, ModeTypeV3,
                },
                portfolio_feed_request::PortfolioUpdateType,
                portfolio_feed_response::PortfolioFeedResponse,
            },
        },
        protos::market_data_feed_v3::FeedResponse as MarketDataFeedV3Response,
        rate_limiter::RateLimitExceeded,
    },
    async_trait::async_trait,
    ezsockets::{Bytes, Client as EzClient, ClientConfig, ClientExt, Error as EzError, Utf8Bytes},
    protobuf::Message,
    reqwest::Url,
    serde::{Deserialize, Serialize},
    serde_json,
    std::collections::{HashSet, hash_set},
    tokio::task::JoinHandle,
};

/// Max concurrent market-data WebSocket connections per Upstox user on
/// the **Upstox Plus** plan.
///
/// Reference: <https://upstox.com/developer/api-documentation/announcements/websocket-plus>
pub const MAX_MARKET_DATA_CONNECTIONS: usize = 5;

/// Max concurrent market-data WebSocket connections per Upstox user on
/// the non-Plus tier. Exposed so operators can assert they are on the
/// correct plan before attempting a Plus-only 5-slot subscribe layout.
pub const MAX_MARKET_DATA_CONNECTIONS_STANDARD: usize = 2;

/// Dense index into the `ApiClient::market_data_feed_v3_clients` pool.
///
/// Prefer the role-keyed API ([`ApiClient::connect_market_data_by_role`] /
/// [`ApiClient::send_market_data_feed_v3_message_by_role`]) when the
/// slot corresponds to one of the five well-known roles defined by
/// [`WsConnectionRole`]; the indexed API is retained for custom
/// topologies that don't map cleanly to the role enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WsConnectionId(pub usize);

impl WsConnectionId {
    /// `true` when this id fits inside the pool (`0..MAX_MARKET_DATA_CONNECTIONS`).
    pub const fn is_valid(self) -> bool {
        self.0 < MAX_MARKET_DATA_CONNECTIONS
    }
}

/// Canonical WS-slot role.
///
/// Each variant has a documented default subscription mode and
/// per-connection key budget derived from the 2026-04-20 Upstox docs.
/// Roles map 1:1 onto the `MAX_MARKET_DATA_CONNECTIONS = 5` pool
/// indices and let callers use `_by_role` sugar instead of raw
/// `WsConnectionId(0..4)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WsConnectionRole {
    /// Nifty 50 constituent equities in `full_d30` (30-level depth).
    ConstituentsD30,
    /// ATM options + near-month futures in `full_d30` (execution zone).
    ExecutionZoneD30,
    /// Full option chain in `full` (5-level depth).
    OptionsChainFull,
    /// Index spot ticks in `ltpc`.
    IndicesLtpc,
    /// Reserved expansion slot (default mode: `full`).
    ExpansionFull,
}

impl WsConnectionRole {
    /// Subscription mode this slot conventionally carries.
    pub const fn default_mode(self) -> ModeTypeV3 {
        match self {
            WsConnectionRole::ConstituentsD30 | WsConnectionRole::ExecutionZoneD30 => {
                ModeTypeV3::FullD30
            }
            WsConnectionRole::OptionsChainFull | WsConnectionRole::ExpansionFull => {
                ModeTypeV3::Full
            }
            WsConnectionRole::IndicesLtpc => ModeTypeV3::LTPC,
        }
    }

    /// Per-connection instrument-key cap for this slot's default mode.
    pub const fn default_max_instruments(self) -> usize {
        match self {
            // `full_d30` cap (Upstox Plus): 50 keys per connection.
            WsConnectionRole::ConstituentsD30 | WsConnectionRole::ExecutionZoneD30 => 50,
            // `full` cap: 2000 keys per connection.
            WsConnectionRole::OptionsChainFull | WsConnectionRole::ExpansionFull => 2000,
            // `ltpc` cap: 5000 keys per connection.
            WsConnectionRole::IndicesLtpc => 5000,
        }
    }

    /// Fixed pool index this role maps to (0..5).
    pub const fn id(self) -> WsConnectionId {
        WsConnectionId(match self {
            WsConnectionRole::ConstituentsD30 => 0,
            WsConnectionRole::ExecutionZoneD30 => 1,
            WsConnectionRole::OptionsChainFull => 2,
            WsConnectionRole::IndicesLtpc => 3,
            WsConnectionRole::ExpansionFull => 4,
        })
    }

    /// Human-readable slot name used for log output and diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            WsConnectionRole::ConstituentsD30 => "constituents_d30",
            WsConnectionRole::ExecutionZoneD30 => "execution_zone_d30",
            WsConnectionRole::OptionsChainFull => "options_chain_full",
            WsConnectionRole::IndicesLtpc => "indices_ltpc",
            WsConnectionRole::ExpansionFull => "expansion_full",
        }
    }
}

/// All 5 market-data slots in fixed id-order. Useful as a fan-out
/// template for the gateway's callback wiring.
pub const ALL_WS_CONNECTION_ROLES: [WsConnectionRole; MAX_MARKET_DATA_CONNECTIONS] = [
    WsConnectionRole::ConstituentsD30,
    WsConnectionRole::ExecutionZoneD30,
    WsConnectionRole::OptionsChainFull,
    WsConnectionRole::IndicesLtpc,
    WsConnectionRole::ExpansionFull,
];

/// Boxed callback signature used for all five market-data pools. Extracted
/// as a type alias so `ApiClient` and `WSConnectConfig` can share a
/// single stable signature.
pub type MarketDataFeedV3CallbackBox = Box<dyn FnMut(MarketDataFeedV3Response) + Send + Sync>;

/// Concrete `ezsockets` client type for one pool slot.
pub type MarketDataFeedV3EzClient = EzClient<MarketDataFeedV3Client<MarketDataFeedV3CallbackBox>>;

/// Array of 5 optional market-data WS clients — one per pool slot.
pub type MarketDataFeedV3ClientPool =
    [Option<MarketDataFeedV3EzClient>; MAX_MARKET_DATA_CONNECTIONS];

#[derive(Debug)]
pub struct PortfolioFeedClient<F>
where
    F: FnMut(PortfolioFeedResponse) + Send + Sync + 'static,
{
    pub handle: EzClient<Self>,
    callback: Option<F>,
}

#[derive(Debug)]
pub struct MarketDataFeedV3Client<F>
where
    F: FnMut(MarketDataFeedV3Response) + Send + Sync + 'static,
{
    pub handle: EzClient<Self>,
    callback: Option<F>,
}

#[async_trait]
impl<F> ClientExt for PortfolioFeedClient<F>
where
    F: FnMut(PortfolioFeedResponse) + Send + Sync + 'static,
{
    type Call = ();

    async fn on_text(&mut self, text: Utf8Bytes) -> Result<(), EzError> {
        if let Some(callback) = &mut self.callback {
            let data: PortfolioFeedResponse = serde_json::from_str::<PortfolioFeedResponse>(&text)?;
            callback(data);
        }
        Ok(())
    }

    async fn on_binary(&mut self, _: Bytes) -> Result<(), EzError> {
        Ok(())
    }

    async fn on_call(&mut self, _: Self::Call) -> Result<(), EzError> {
        Ok(())
    }
}

#[async_trait]
impl<F> ClientExt for MarketDataFeedV3Client<F>
where
    F: FnMut(MarketDataFeedV3Response) + Send + Sync + 'static,
{
    type Call = MarketDataV3Call;

    async fn on_text(&mut self, _: Utf8Bytes) -> Result<(), EzError> {
        Ok(())
    }

    async fn on_binary(&mut self, binary_data: Bytes) -> Result<(), EzError> {
        if let Some(callback) = &mut self.callback {
            let data: MarketDataFeedV3Response =
                MarketDataFeedV3Response::parse_from_bytes(&binary_data)?;
            callback(data);
        }
        Ok(())
    }

    async fn on_call(&mut self, call: Self::Call) -> Result<(), EzError> {
        let market_data_feed_message: MarketDataFeedV3Message = MarketDataFeedV3Message {
            guid: "someguid".to_string(),
            method: match &call {
                MarketDataV3Call::SubscribeInstrument(_) => MethodTypeV3::Sub,
                MarketDataV3Call::ChangeMode(_) => MethodTypeV3::ChangeMode,
                MarketDataV3Call::UnsubscribeInstrument(_) => MethodTypeV3::Unsub,
            },
            data: match call {
                MarketDataV3Call::SubscribeInstrument(data) => data,
                MarketDataV3Call::ChangeMode(data) => data,
                MarketDataV3Call::UnsubscribeInstrument(data) => data,
            },
        };

        let message_text: String = serde_json::to_string(&market_data_feed_message).unwrap();
        let message_binary: Vec<u8> = message_text.into_bytes();
        self.handle.binary(message_binary)?;
        Ok(())
    }
}

impl ApiClient {
    // Default update type is order only
    pub async fn connect_portfolio_feed(
        &mut self,
        update_types: Option<HashSet<PortfolioUpdateType>>,
        callback: Option<Box<dyn FnMut(PortfolioFeedResponse) + Send + Sync>>,
    ) -> Result<JoinHandle<()>, String> {
        let authorized_url: String = self
            .get_authorized_portfolio_feed_endpoint(update_types)
            .await
            .map_err(|e| format!("rate-limit / transport error: {e:?}"))?
            .map_err(|_| "Failed to fetch Portfolio Feed WS URL".to_string())?
            .data
            .authorized_redirect_uri;

        let config: ClientConfig = ClientConfig::new(Url::parse(&authorized_url).unwrap());
        let (handle, future) =
            ezsockets::connect(|handle| PortfolioFeedClient { handle, callback }, config).await;
        self.portfolio_feed_client = Some(handle);

        let feed_future: JoinHandle<()> = tokio::spawn(async move {
            future.await.unwrap();
        });
        Ok(feed_future)
    }

    /// Open a market-data WebSocket for the given pool slot.
    ///
    /// Each slot is independent — opening slot 0 has no effect on slot
    /// 3, and each can carry its own subscribe/unsubscribe traffic.
    /// The callback fires for every protobuf `FeedResponse` arriving
    /// on that slot.
    ///
    /// Returns an error when:
    /// - `conn.0 >= MAX_MARKET_DATA_CONNECTIONS`,
    /// - slot `conn` is already connected
    ///   (use [`ApiClient::is_market_data_connected`] first), or
    /// - `conn.0 >= MAX_MARKET_DATA_CONNECTIONS_STANDARD` (= 2) AND
    ///   the client was built without `ClientCapabilities::is_plus_user`
    ///   — non-Plus accounts cap at 2 physical sockets.
    pub async fn connect_market_data_feed_v3(
        &mut self,
        conn: WsConnectionId,
        callback: Option<MarketDataFeedV3CallbackBox>,
    ) -> Result<JoinHandle<()>, String> {
        if !conn.is_valid() {
            return Err(format!(
                "WsConnectionId {} is out of range (max {MAX_MARKET_DATA_CONNECTIONS})",
                conn.0
            ));
        }
        let cap = self.capabilities.max_market_data_connections();
        if conn.0 >= cap {
            return Err(format!(
                "WsConnectionId {} exceeds the {cap}-connection cap for this \
                 account (is_plus_user = {}). Upstox Plus unlocks up to \
                 MAX_MARKET_DATA_CONNECTIONS = {MAX_MARKET_DATA_CONNECTIONS}.",
                conn.0, self.capabilities.is_plus_user,
            ));
        }
        if self.market_data_feed_v3_clients[conn.0].is_some() {
            return Err(format!(
                "market-data WS slot {} is already connected — disconnect it first",
                conn.0
            ));
        }

        let authorized_url: String = self
            .get_authorized_market_data_feed_v3_endpoint()
            .await
            .map_err(|e| format!("rate-limit / transport error: {e:?}"))?
            .map_err(|_| "Failed to fetch Market Data Feed V3 WS URL".to_string())?
            .data
            .authorized_redirect_uri;

        let config: ClientConfig = ClientConfig::new(Url::parse(&authorized_url).unwrap());
        let (handle, future) =
            ezsockets::connect(|handle| MarketDataFeedV3Client { handle, callback }, config).await;
        self.market_data_feed_v3_clients[conn.0] = Some(handle);

        let feed_future: JoinHandle<()> = tokio::spawn(async move {
            future.await.unwrap();
        });
        Ok(feed_future)
    }

    /// Convenience wrapper — open the physical slot this role maps to
    /// via [`WsConnectionRole::id`].
    pub async fn connect_market_data_by_role(
        &mut self,
        role: WsConnectionRole,
        callback: Option<MarketDataFeedV3CallbackBox>,
    ) -> Result<JoinHandle<()>, String> {
        self.connect_market_data_feed_v3(role.id(), callback).await
    }

    /// `true` when the given pool slot currently holds an active WS client.
    pub fn is_market_data_connected(&self, conn: WsConnectionId) -> bool {
        conn.is_valid() && self.market_data_feed_v3_clients[conn.0].is_some()
    }

    /// Send a subscribe / change-mode / unsubscribe message on the
    /// specified pool slot.
    ///
    /// Refuses `full_d30` subscribes / change-modes with a descriptive
    /// error when the client was built without
    /// `ClientCapabilities::is_plus_user` — the broker silently
    /// rejects those subscribes on the standard tier, so a local
    /// refusal gives operators a much cleaner failure signal.
    ///
    /// The call is silently dropped when the slot is not connected; use
    /// [`ApiClient::is_market_data_connected`] to distinguish "slot
    /// empty" from "slot failed to send" (the latter surfaces as an
    /// `Err(EzError)`).
    pub async fn send_market_data_feed_v3_message(
        &self,
        conn: WsConnectionId,
        market_data_feed_v3_message: MarketDataV3Call,
    ) -> Result<(), EzError> {
        if !conn.is_valid() {
            return Err(format!(
                "WsConnectionId {} is out of range (max {MAX_MARKET_DATA_CONNECTIONS})",
                conn.0
            )
            .into());
        }
        if !self.capabilities.is_plus_user && call_uses_full_d30(&market_data_feed_v3_message) {
            return Err(format!(
                "ModeTypeV3::FullD30 subscribe on slot {} requires Upstox Plus \
                 (set ClientCapabilities::is_plus_user = true)",
                conn.0,
            )
            .into());
        }
        if let Some(client) = &self.market_data_feed_v3_clients[conn.0] {
            client.call(market_data_feed_v3_message)?;
        }
        Ok(())
    }

    /// Convenience wrapper — send on the slot this role maps to.
    pub async fn send_market_data_feed_v3_message_by_role(
        &self,
        role: WsConnectionRole,
        call: MarketDataV3Call,
    ) -> Result<(), EzError> {
        self.send_market_data_feed_v3_message(role.id(), call).await
    }

    pub async fn get_authorized_portfolio_feed_endpoint(
        &self,
        update_types: Option<HashSet<PortfolioUpdateType>>,
    ) -> Result<Result<SuccessResponse<AuthorizeFeedResponse>, ErrorResponse>, RateLimitExceeded>
    {
        let update_types: String = match update_types {
            Some(types) => {
                if types.is_empty() {
                    "order".to_string()
                } else {
                    let mut iter: hash_set::Iter<PortfolioUpdateType> = types.iter();
                    let mut temp: String = iter.next().unwrap().to_string();
                    for val in iter {
                        temp.push_str(",");
                        temp.push_str(&val.to_string());
                    }
                    temp
                }
            }
            None => "order".to_string(),
        };

        let res: reqwest::Response = self
            .get(
                WS_PORTFOLIO_FEED_AUTHORIZE_ENDPOINT,
                true,
                Some(&vec![("update_types".to_string(), update_types)]),
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<AuthorizeFeedResponse>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }

    pub async fn get_authorized_market_data_feed_v3_endpoint(
        &self,
    ) -> Result<Result<SuccessResponse<AuthorizeFeedResponse>, ErrorResponse>, RateLimitExceeded>
    {
        let res: reqwest::Response = self
            .get(
                WS_MARKET_DATA_FEED_AUTHORIZE_ENDPOINT,
                true,
                None,
                BaseUrlType::REGULAR,
                APIVersion::V3,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<AuthorizeFeedResponse>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }
}

#[derive(Debug)]
pub enum MarketDataV3Call {
    SubscribeInstrument(MessageDataV3),
    ChangeMode(MessageDataV3),
    UnsubscribeInstrument(MessageDataV3),
    // Add other calls as needed
}

/// `true` when the call carries a `FullD30` mode in its payload on any
/// variant. Used by [`ApiClient::send_market_data_feed_v3_message`]
/// to refuse Plus-only subscribes up-front on non-Plus clients.
fn call_uses_full_d30(call: &MarketDataV3Call) -> bool {
    let data = match call {
        MarketDataV3Call::SubscribeInstrument(d)
        | MarketDataV3Call::ChangeMode(d)
        | MarketDataV3Call::UnsubscribeInstrument(d) => d,
    };
    matches!(data.mode, ModeTypeV3::FullD30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_connection_role_ids_cover_zero_through_four_exactly_once() {
        let mut seen = [false; MAX_MARKET_DATA_CONNECTIONS];
        for role in ALL_WS_CONNECTION_ROLES {
            let WsConnectionId(idx) = role.id();
            assert!(idx < MAX_MARKET_DATA_CONNECTIONS);
            assert!(!seen[idx], "duplicate id for {role:?}");
            seen[idx] = true;
        }
        assert!(
            seen.iter().all(|&v| v),
            "missing slot in role -> id mapping"
        );
    }

    #[test]
    fn ws_connection_role_default_modes_match_docs() {
        assert_eq!(
            WsConnectionRole::ConstituentsD30.default_mode(),
            ModeTypeV3::FullD30
        );
        assert_eq!(
            WsConnectionRole::ExecutionZoneD30.default_mode(),
            ModeTypeV3::FullD30
        );
        assert_eq!(
            WsConnectionRole::OptionsChainFull.default_mode(),
            ModeTypeV3::Full
        );
        assert_eq!(
            WsConnectionRole::IndicesLtpc.default_mode(),
            ModeTypeV3::LTPC
        );
        assert_eq!(
            WsConnectionRole::ExpansionFull.default_mode(),
            ModeTypeV3::Full
        );
    }

    #[test]
    fn ws_connection_role_default_max_instruments_match_upstox_plus() {
        assert_eq!(
            WsConnectionRole::ConstituentsD30.default_max_instruments(),
            50
        );
        assert_eq!(
            WsConnectionRole::ExecutionZoneD30.default_max_instruments(),
            50
        );
        assert_eq!(
            WsConnectionRole::OptionsChainFull.default_max_instruments(),
            2000
        );
        assert_eq!(
            WsConnectionRole::IndicesLtpc.default_max_instruments(),
            5000
        );
        assert_eq!(
            WsConnectionRole::ExpansionFull.default_max_instruments(),
            2000
        );
    }

    #[test]
    fn ws_connection_id_validates_range() {
        assert!(WsConnectionId(0).is_valid());
        assert!(WsConnectionId(4).is_valid());
        assert!(!WsConnectionId(5).is_valid());
        assert!(!WsConnectionId(999).is_valid());
    }

    #[test]
    fn max_connections_matches_upstox_plus_spec() {
        assert_eq!(MAX_MARKET_DATA_CONNECTIONS, 5);
        assert_eq!(MAX_MARKET_DATA_CONNECTIONS_STANDARD, 2);
    }

    #[test]
    fn call_uses_full_d30_detects_every_variant() {
        let data_d30 = MessageDataV3 {
            mode: ModeTypeV3::FullD30,
            instrument_keys: vec!["x".into()],
        };
        assert!(call_uses_full_d30(&MarketDataV3Call::SubscribeInstrument(
            data_d30.clone()
        )));
        assert!(call_uses_full_d30(&MarketDataV3Call::ChangeMode(
            data_d30.clone()
        )));
        assert!(call_uses_full_d30(
            &MarketDataV3Call::UnsubscribeInstrument(data_d30)
        ));

        let data_full = MessageDataV3 {
            mode: ModeTypeV3::Full,
            instrument_keys: vec!["x".into()],
        };
        assert!(!call_uses_full_d30(&MarketDataV3Call::SubscribeInstrument(
            data_full
        )));
    }
}
