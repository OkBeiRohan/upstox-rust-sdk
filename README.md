# Upstox Rust SDK

## Introduction

A Rust client for communicating with the <a href="https://upstox.com/uplink/">Upstox API</a>.

Upstox API is a set of rest APIs that provide data required to build a complete investment and trading platform. Execute orders in real time, manage user portfolio, stream live market data (using Websocket), and a lot more with this crate.

### What's new in `2.0` (see `CHANGELOG.md`)

- **Parallel market-data WebSocket connections** via `WsConnectionRole` / `WsConnectionId`. Up to 5 physical sockets on Upstox Plus, 2 on the standard tier — enforced by the SDK based on [`ClientCapabilities::is_plus_user`](src/client.rs).
- **Native `full_d30` mode** — `ModeTypeV3::FullD30` with a pinned wire name and snapshot-tested subscribe envelope. Rejected at the SDK layer for non-Plus accounts.
- **Capability-aware feature gating** — `ClientCapabilities { is_plus_user, is_sebi_registered }` fails fast with `RateLimitExceeded::FeatureRequiresPlus(...)` when non-Plus code paths attempt Plus-only APIs (extra WS slots, `full_d30`, expired-instruments endpoints).
- **Shared-bucket rate limiter** that matches the Upstox docs (`OrderPlacement` + `Standard` buckets, 10/50 per-sec order cap depending on `RateLimitProfile::{RegularAlgo, SebiRegistered}`). No more per-endpoint `(25, 250, 1000)` fiction.
- **Permissive `Exchange` parsing** — `NSCOM` + `Exchange::Other(String)` so future exchange codes never panic JSON decode.
- **2026-03/04 API additions** — `market_protection` on every order request, `trailing_gap` on GTT rules, `X-Algo-Name` header auto-injected on order-bucket endpoints, `get_fund_and_margin_v3` with the nested cash/pledge split.
- **No more hot-path panics** — `request.send().await` and `"Unsupported HTTP Method"` now surface as `RateLimitExceeded::{Network, UnsupportedMethod}` variants (the outer error enum is `#[non_exhaustive]`).

## Requirements
- Install `libssl-dev` on Linux

## Environment Variables

These environment variables are used optionally in the SDK depending on the features to be used.

- EMAIL_ID: Email used for Upstox account like "abc@example.com" (Only needed when automating login).
- GOOGLE_AUTHORIZATION_CODE: Authorization code obtained upon Google OAuth 2.0 Authentication which expires in 1 hr. Provide newly fetched value only when manual login page is needed to be skipped (Only needed when automating fetching OTP and using Gmail).
- GOOGLE_CLIENT_ID: Google Client ID for Google Gmail API access (Only needed when automating fetching OTP and using Gmail).
- GOOGLE_CLIENT_SECRET: Google Client Secret for Google Gmail API access (Only needed when automating fetching OTP and using Gmail).
- MOBILE_NUMBER: Mobile number used for Upstox account (Only needed when automating login).
- LOGIN_PIN: Login PIN for Upstox account (Only needed when automating login).
- REDIRECT_PORT: The local port used for redirection for both Upstox API and Gmail API like 8080. Redirect URL provided to both Upstox and Google must be "http://127.0.0.1:$REDIRECT_PORT" if login is needed for authorized endpoint access.
- UPLINK_API_KEY: Upstox API Key. Required for authorized API access ([`Generate Here`](https://account.upstox.com/developer/apps)).
- UPLINK_API_SECRET: Upstox API Secret. Required for authorized API access ([`Generate Here`](https://account.upstox.com/developer/apps)).
- WEBDRIVER_SOCKET: The local socket on which chromedriver or geckodriver is running. They run by default on "http://127.0.0.1:4444" (Only needed when automating login).


## Examples

- [`login-usage`](examples/login_usage): Example on using login functionality to get access token, automating login, fetching OTP automatically, scheduling automatic re-login.
- [`fetch-instruments`](examples/fetch_instruments): Example on fetching available instruments on startup and refreshing them daily.
- [`ws-usage`](examples/ws_usage): Two-slot WebSocket fan-out — subscribes one slot in `Full` mode and another in `full_d30`.
- [`ws-multi`](examples/ws_multi): Full 5-slot Upstox Plus fan-out — one physical WS per `WsConnectionRole`, each with its own subscribe batch and role-stamped callback.
- [`api-usage`](examples/api_usage): Example on using Upstox's REST API endpoints via the ApiClient.

## License

Licensed under <a href="https://choosealicense.com/licenses/mpl-2.0/">MPL 2.0</a>


## Contact

Reach out by mailing me at aviralomar0301@gmail.com
