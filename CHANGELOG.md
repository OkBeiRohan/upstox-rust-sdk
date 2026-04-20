# Changelog

All notable changes to `upstox-rust-sdk`.

## 2.0.0 — 2026-04-20

Major refactor that replaces the single-WebSocket, per-endpoint rate-limit
model with the account-capability-aware design Upstox's 2025–2026 docs
describe.

### Breaking

- `WSConnectConfig` now holds a `market_data_streams: Vec<WsChannelConfig>` instead of the scalar `connect_market_data_stream_v3: bool` + `market_data_feed_v3_callback: Option<Box<...>>` pair. Each entry pins a `WsConnectionId` so the SDK can open up to `MAX_MARKET_DATA_CONNECTIONS = 5` parallel market-data WebSockets on Upstox Plus (2 on the standard tier).
- `ApiClient::market_data_feed_v3_client` (scalar `Option<...>`) was removed. The pool now lives behind `market_data_feed_v3_clients: MarketDataFeedV3ClientPool` (`[Option<EzClient<...>>; 5]`).
- `ApiClient::connect_market_data_feed_v3(&mut self, conn: WsConnectionId, callback)` replaces the previous `(&mut self, callback)` signature.
- `ApiClient::send_market_data_feed_v3_message(&self, conn: WsConnectionId, call)` takes the target pool slot as a first argument.
- `RateLimitExceeded` is now `#[non_exhaustive]` with new `Network(String)`, `UnsupportedMethod(String)`, and `FeatureRequiresPlus(String)` variants — exhaustive matches need a wildcard arm.
- `ApiRateLimiter::new(RateLimitProfile)` replaces the per-endpoint `(per_second, per_minute, per_thirty_minutes)` constructor. Pre-v2 instantiated a fresh `(25, 250, 1000)` bucket per endpoint URL; v2 keeps two shared buckets (`OrderPlacement` + `Standard`) that match the Upstox docs exactly.
- `ExitAllPositionsRequest` and `CancelMultiOrderRequest` fix the historical `taget` typo to `tag` so the broker actually honours the filter.
- `get_exchange_status`'s parameter is now spelled `exchange_status_path_params` (was `exchange_staus_path_params`).

### Added

- **Native `full_d30` support** — `ModeTypeV3::FullD30` (wire name `"full_d30"`); proto `RequestMode::full_d30` was already decoded. Snapshot test pins the subscribe envelope byte-for-byte.
- **Parallel market-data WebSocket connections** — `WsConnectionId` (pool index 0..5), `WsConnectionRole` enum (`ConstituentsD30`, `ExecutionZoneD30`, `OptionsChainFull`, `IndicesLtpc`, `ExpansionFull`) with `id()`, `default_mode()`, `default_max_instruments()`, and `name()` helpers. Convenience sugar: `connect_market_data_by_role`, `send_market_data_feed_v3_message_by_role`. `is_market_data_connected(id)` reports per-slot state.
- **`ClientCapabilities { is_plus_user, is_sebi_registered }`** — account-capability flags the SDK enforces at the feature boundary. Non-Plus clients are rejected with `RateLimitExceeded::FeatureRequiresPlus` when they try to open more than 2 market-data WebSockets, subscribe in `full_d30` mode, or hit any of the expired-instruments endpoints. `is_sebi_registered` drives the order-bucket per-second cap (10/s non-registered → 50/s registered).
- **`ApiClient::new_with_capabilities`** — primary configurable constructor. `ApiClient::new` keeps the one-liner default (non-Plus, non-SEBI).
- **Shared-bucket rate limiter** matching the Upstox docs — `RateLimitBucket::{OrderPlacement, Standard}`, `RateLimitProfile::{RegularAlgo, SebiRegistered}`, plus `classify_endpoint(&str) -> RateLimitBucket` for downstream reuse. Constants: `ORDER_BUCKET_PER_SECOND_REGULAR = 10`, `ORDER_BUCKET_PER_SECOND_SEBI = 50`, `ORDER_BUCKET_PER_MINUTE = 500`, `ORDER_BUCKET_PER_30_MINUTES = 2000`, `STANDARD_BUCKET_*`.
- **Permissive `Exchange` parsing** — `Exchange::NSCOM` variant (Upstox started returning it in April 2026) plus a `Exchange::Other(String)` fallthrough so future exchange codes never panic decode. `#[serde(from = "String", into = "String")]` preserves round-trip fidelity.
- **`market_protection: Option<i32>`** on `PlaceOrderV3Request`, `PlaceOrderRequest`, `ModifyOrderRequest`, `PlaceMultiOrderRequest`, `ExitAllPositionsRequest`, and `GTTOrderRule`. Validator accepts `-1` (auto), `0` (none), or `1..=25` (custom percent). Reference: 2026-03-11 Upstox announcement.
- **`X-Algo-Name` header injection** — `ApiClient::set_algo_name(Option<String>)` sets the value; header is attached automatically on order-placement-bucket endpoints only. Reference: 2026-04-01 SEBI/Exchange circular on registered algos.
- **Get Fund and Margin V3** — `get_fund_and_margin_v3` + `FundAndMarginV3Response` with nested `available_to_trade.{cash,pledge}` and `unavailable_to_trade.*` breakdowns. Reference: 2026-04-10 Upstox announcement.
- Re-exports at crate root: `ClientCapabilities`, `RateLimitBucket`, `RateLimitExceeded`, `RateLimitProfile`, `WsConnectionId`, `WsConnectionRole`, `MarketDataV3Call`, `MAX_MARKET_DATA_CONNECTIONS`, `MAX_MARKET_DATA_CONNECTIONS_STANDARD`, `ALL_WS_CONNECTION_ROLES`.

### Changed

- `PlaceOrderV3Request::instrument_token`, `PlaceOrderRequest::instrument_token`, `PlaceMultiOrderRequest::instrument_token`, and `PlaceGTTOrderRequest::instrument_token` regexes now accept `NSE_COM`, `BSE_COM`, `NCD_COM`, and `MCX_COM`. Pre-v2 the SDK had an `ExchangeSegment::NseCom` variant but the regex rejected the matching token.
- `ApiClient::request`'s `request.send().await.unwrap()` is replaced with `?` propagation via the new `RateLimitExceeded::Network` variant. Transient reqwest failures no longer abort the process.
- `ApiClient::request`'s `panic!("Unsupported HTTP Method")` is replaced with the new `RateLimitExceeded::UnsupportedMethod` variant.
- `ModeTypeV3` variants now carry explicit `#[serde(rename = "...")]` annotations so the wire names stay pinned regardless of serde's snake-case heuristics.

### Deprecated

- `RATE_LIMIT_PER_SECOND = 25`, `RATE_LIMIT_PER_MINUTE = 250`, `RATE_LIMIT_PER_THIRTY_MINUTES = 1000` (kept as `#[deprecated]` aliases for one release).
- `ApiClient::get_fund_and_margin` — use `get_fund_and_margin_v3` instead. The V2 response shape changed on 2025-07-19 (combined equity + commodity in the `equity` object) and V3 exposes the cash/pledge split the 2026-04-10 rollout added.
- `ApiClient::new_with_profile` — use `new_with_capabilities` which accepts the full capability set.
