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
    ezsockets::{
        Bytes, Client as EzClient, ClientConfig, ClientExt, Error as EzError, Utf8Bytes,
        client::ClientCloseMode,
    },
    protobuf::Message,
    reqwest::Url,
    serde::{Deserialize, Serialize},
    serde_json,
    std::{
        collections::{HashSet, hash_set},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    },
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

/// Slot pool entry — pairs the ezsockets handle with a shared
/// `is_open` flag that the slot's `ClientExt` flips in
/// `on_connect` / `on_disconnect`. Without this pair, callers
/// that ask "is the WS open?" only learn whether the handle was
/// stored (i.e. `ezsockets::connect` returned), NOT whether the
/// TLS + WS-upgrade handshake has completed. The race that fix
/// resolves: NOB's `subscribe_plan` fires the moment all 5 slots'
/// connect calls return, and used to dispatch `Sub` frames into
/// not-yet-open sockets — ezsockets dropped them silently and
/// the broker reported a successful subscribe with zero ticks
/// arriving on the affected slot until the next reconnect.
///
/// `Debug` is intentionally NOT derived: the inner ezsockets
/// `Client` carries a `dyn FnMut(...)` callback that does not
/// implement `Debug`, and the field is opaque to callers anyway.
pub struct MarketDataFeedV3PoolEntry {
    pub handle: MarketDataFeedV3EzClient,
    pub is_open: Arc<AtomicBool>,
}

impl std::fmt::Debug for MarketDataFeedV3PoolEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarketDataFeedV3PoolEntry")
            .field("handle", &"<EzClient>")
            .field("is_open", &self.is_open.load(Ordering::Acquire))
            .finish()
    }
}

/// Array of 5 optional market-data WS clients — one per pool slot.
pub type MarketDataFeedV3ClientPool =
    [Option<MarketDataFeedV3PoolEntry>; MAX_MARKET_DATA_CONNECTIONS];

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
    /// Shared with `MarketDataFeedV3PoolEntry::is_open` so callers
    /// can poll true WS-open state. ezsockets drives this via
    /// `on_connect` (set true) and `on_disconnect` (set false).
    is_open: Arc<AtomicBool>,
}

#[async_trait]
impl<F> ClientExt for PortfolioFeedClient<F>
where
    F: FnMut(PortfolioFeedResponse) + Send + Sync + 'static,
{
    type Call = ();

    async fn on_text(&mut self, text: Utf8Bytes) -> Result<(), EzError> {
        // Resilience: a malformed portfolio JSON frame (or a new
        // SDK-untaught variant) must NOT force-close the portfolio
        // WS. Log + drop and keep the stream alive for the next
        // order / position update.
        if let Some(callback) = &mut self.callback {
            match serde_json::from_str::<PortfolioFeedResponse>(&text) {
                Ok(data) => callback(data),
                Err(e) => tracing::warn!(
                    error = %e,
                    bytes = text.len(),
                    "portfolio-feed JSON parse failed; dropping frame (WS stays open)"
                ),
            }
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

    /// Flip the shared open-flag on TLS+WS upgrade success.
    /// `is_market_data_connected` reads this so callers waiting on
    /// the slot can distinguish "handle stored" from "socket actually
    /// open and able to ship `Sub` frames".
    async fn on_connect(&mut self) -> Result<(), EzError> {
        self.is_open.store(true, Ordering::Release);
        Ok(())
    }

    /// Mirror of `on_connect` — the open-flag flips back to false on
    /// any disconnect (transient, broker-initiated, or fatal). The
    /// supervisor `ClientCloseMode::Reconnect` keeps ezsockets's
    /// auto-reconnect behaviour intact; the next successful
    /// `on_connect` re-arms the flag.
    async fn on_disconnect(&mut self) -> Result<ClientCloseMode, EzError> {
        self.is_open.store(false, Ordering::Release);
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_text(&mut self, _: Utf8Bytes) -> Result<(), EzError> {
        Ok(())
    }

    async fn on_binary(&mut self, binary_data: Bytes) -> Result<(), EzError> {
        // CRITICAL: per ezsockets `ClientExt` semantics, returning
        // `Err` here force-closes the WS. A single malformed or
        // newly-typed protobuf frame from Upstox (which we have
        // observed in production whenever the broker rolls a feature
        // flag) would therefore kill the slot for the rest of the
        // session — and our pool entry would stay alive until the
        // 90 s stall watchdog noticed. Swallow the parse error
        // instead: log + drop the offending frame, keep the WS up
        // for the next batch.
        if let Some(callback) = &mut self.callback {
            match MarketDataFeedV3Response::parse_from_bytes(&binary_data) {
                Ok(data) => callback(data),
                Err(e) => tracing::warn!(
                    error = %e,
                    bytes = binary_data.len(),
                    "market-data protobuf parse failed; dropping batch (WS stays open)"
                ),
            }
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
        // Same rationale as `on_binary`: if the underlying socket
        // is mid-reconnect, `handle.binary(...)` returns `Err` and
        // returning it here would force-close the WS — exactly the
        // wrong response when the actor is already trying to
        // recover. Log + drop instead so the next subscribe arrives
        // on the freshly-reconnected socket.
        if let Err(e) = self.handle.binary(message_binary) {
            tracing::warn!(
                error = %e,
                "market-data WS binary send failed (likely mid-reconnect); \
                 dropping subscribe frame, caller should re-issue after reconnect"
            );
        }
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

        // Drive the actor-loop future without `unwrap()`: a panic
        // here would tear down the spawned task with no operator
        // signal. Log the exit instead so a portfolio-feed death
        // (which silently stops fill / position updates from
        // reaching the executor) surfaces in the logs.
        let feed_future: JoinHandle<()> = tokio::spawn(async move {
            match future.await {
                Ok(()) => tracing::warn!(
                    "portfolio-feed WS actor exited cleanly \
                     (no further fills / positions will reach the executor)"
                ),
                Err(e) => tracing::error!(
                    error = %e,
                    "portfolio-feed WS actor exited with error \
                     (no further fills / positions will reach the executor)"
                ),
            }
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
        // Share an `is_open` flag between the slot's `ClientExt` and
        // the pool entry. The flag stays `false` until ezsockets
        // dispatches `on_connect` (TLS + WS upgrade complete) and
        // flips back to `false` on every `on_disconnect`. Callers MUST
        // poll this through `is_market_data_connected` before
        // dispatching subscribes — bare handle existence is not enough.
        let is_open = Arc::new(AtomicBool::new(false));
        let is_open_for_client = Arc::clone(&is_open);
        let (handle, future) = ezsockets::connect(
            move |handle| MarketDataFeedV3Client {
                handle,
                callback,
                is_open: is_open_for_client,
            },
            config,
        )
        .await;
        self.market_data_feed_v3_clients[conn.0] = Some(MarketDataFeedV3PoolEntry {
            handle,
            is_open: Arc::clone(&is_open),
        });

        // Drive the actor-loop future to completion in a dedicated
        // task. CRITICAL: do NOT `unwrap()` the result here — the
        // future returns `Err(...)` on persistent connect failure,
        // unrecoverable handshake error, or operator-initiated close,
        // and a panic in the spawned task would leave the slot's
        // pool entry ALIVE (`is_open == true`) while the underlying
        // ezsockets actor is dead, invisible to every consumer of
        // [`Self::is_market_data_connected`].
        //
        // Instead: on ANY exit (clean or error), flip `is_open` to
        // false so the watchdog observes the slot as disconnected
        // and can dispatch a reconnect via
        // [`Self::reconnect_market_data_by_id`]. The error is
        // logged with the slot id so operators see *which* slot
        // died.
        let conn_id_for_log = conn.0;
        let feed_future: JoinHandle<()> = tokio::spawn(async move {
            match future.await {
                Ok(()) => tracing::warn!(
                    slot = conn_id_for_log,
                    "market-data WS actor exited cleanly (slot now silent until reconnect)"
                ),
                Err(e) => tracing::error!(
                    slot = conn_id_for_log,
                    error = %e,
                    "market-data WS actor exited with error (slot now silent until reconnect)"
                ),
            }
            // Flip BEFORE the task returns so the watchdog's next
            // `is_market_data_connected` poll observes the dead state.
            is_open.store(false, Ordering::Release);
        });
        Ok(feed_future)
    }

    /// Tear down a dead market-data slot's pool entry and re-spawn a
    /// fresh WebSocket on the same role with the supplied callback.
    ///
    /// Rationale: when [`Self::connect_market_data_feed_v3`]'s
    /// underlying actor task exits (the handler returned `Err`,
    /// ezsockets exhausted reconnect attempts, etc.), the pool
    /// entry stays around with `is_open == false` so consumers can
    /// detect the death — but the dead handle is unusable. This
    /// helper drops the dead entry and runs the connect path again.
    ///
    /// Returns the new actor-loop `JoinHandle`. Callers should hold
    /// it for the lifetime of the slot or `tokio::spawn` a
    /// supervisor; on the SDK boot path that's
    /// [`ApiClient::new_with_capabilities`] which keeps every
    /// returned handle inside its `tasks_vec`.
    pub async fn reconnect_market_data_by_id(
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
        // Drop the dead pool entry first so `connect_market_data_feed_v3`'s
        // "already connected" guard does not refuse the fresh open.
        // Any clones of the previous handle held by callers will
        // continue to send into the dead async-channel until they
        // are dropped — that's harmless because the actor loop is
        // gone and the channel is unbounded; messages just queue
        // and never get processed.
        self.market_data_feed_v3_clients[conn.0] = None;
        self.connect_market_data_feed_v3(conn, callback).await
    }

    /// Convenience wrapper: reconnect by [`WsConnectionRole`].
    pub async fn reconnect_market_data_by_role(
        &mut self,
        role: WsConnectionRole,
        callback: Option<MarketDataFeedV3CallbackBox>,
    ) -> Result<JoinHandle<()>, String> {
        self.reconnect_market_data_by_id(role.id(), callback).await
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

    /// `true` when the given pool slot's WebSocket has finished the
    /// TLS + WS-upgrade handshake and is ready to ship `Sub` /
    /// `ChangeMode` / `Unsub` frames.
    ///
    /// Reads the shared `is_open` flag the slot's `ClientExt` flips
    /// in `on_connect` / `on_disconnect`. This is intentionally
    /// stricter than "is the handle stored?" — a handle that exists
    /// but whose underlying socket is mid-handshake will silently
    /// drop subscribe traffic (per ezsockets semantics), so callers
    /// must wait for this to return `true` before issuing the first
    /// `Sub` frame on any new slot.
    pub fn is_market_data_connected(&self, conn: WsConnectionId) -> bool {
        conn.is_valid()
            && self.market_data_feed_v3_clients[conn.0]
                .as_ref()
                .is_some_and(|entry| entry.is_open.load(Ordering::Acquire))
    }

    /// `true` when the given pool slot has a stored handle, regardless
    /// of WS-open status. Useful for "should I attempt a connect?"
    /// checks where the answer is "no, it's already in flight".
    /// Most callers want [`Self::is_market_data_connected`] instead.
    pub fn has_market_data_handle(&self, conn: WsConnectionId) -> bool {
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
        // Refuse to dispatch into a slot whose WS upgrade has not
        // completed yet. ezsockets's `client.call()` would buffer or
        // silently drop the frame depending on its internal state,
        // and the broker would acknowledge a subscribe that never
        // actually arrived. Returning a typed error here gives the
        // caller a clear failure signal it can retry on instead of
        // declaring success for a phantom subscribe.
        match self.market_data_feed_v3_clients[conn.0].as_ref() {
            Some(entry) if entry.is_open.load(Ordering::Acquire) => {
                entry.handle.call(market_data_feed_v3_message)?;
                Ok(())
            }
            Some(_) => Err(format!(
                "market-data WS slot {} handle exists but socket is not open yet \
                 (TLS / WS upgrade still in flight) — caller must wait for \
                 `is_market_data_connected` before dispatching",
                conn.0
            )
            .into()),
            None => Err(format!(
                "market-data WS slot {} is not connected — call \
                 `connect_market_data_feed_v3` first",
                conn.0
            )
            .into()),
        }
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
