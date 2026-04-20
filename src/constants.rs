pub(super) enum BaseUrlType {
    REGULAR,
    HFT,
    SANDBOX,
}

pub(super) enum APIVersion {
    V2,
    V3,
}

pub(super) const LOGIN_AUTHORIZE_ENDPOINT: &str = "/login/authorization/dialog";
pub(super) const LOGIN_GET_TOKEN_ENDPOINT: &str = "/login/authorization/token";
pub(super) const LOGOUT_ENDPOINT: &str = "/logout";

pub(super) const INSTRUMENTS_COMPLETE_URL: &str =
    "https://assets.upstox.com/market-quote/instruments/exchange/complete.json.gz";

pub(super) const USER_GET_FUND_AND_MARGIN_ENDPOINT: &str = "/user/get-funds-and-margin";
/// V3 funds/margin endpoint (2026-04-10). Same path as V2 — the SDK
/// differentiates by calling `create_url` with `APIVersion::V3`.
pub(super) const USER_GET_FUND_AND_MARGIN_V3_ENDPOINT: &str = "/user/get-funds-and-margin";
pub(super) const USER_GET_PROFILE_ENDPOINT: &str = "/user/profile";

pub(super) const CHARGES_BROKERAGE_DETAILS_ENDPOINT: &str = "/charges/brokerage";
pub(super) const MARGINS_MARGIN_DETAILS_ENDPOINT: &str = "/charges/margin";

pub(super) const ORDERS_PLACE_ORDER_ENDPOINT: &str = "/order/place";
pub(super) const ORDERS_PLACE_MULTI_ORDER_ENDPOINT: &str = "/order/multi/place";
pub(super) const ORDERS_MODIFY_ORDER_ENDPOINT: &str = "/order/modify";
pub(super) const ORDERS_CANCEL_ORDER_ENDPOINT: &str = "/order/cancel";
pub(super) const ORDERS_CANCEL_MULTI_ORDER_ENDPOINT: &str = "/order/multi/cancel";
pub(super) const ORDERS_EXIT_ALL_POSITIONS_ENDPOINT: &str = "/order/positions/exit";
pub(super) const ORDERS_ORDER_DETAILS_ENDPOINT: &str = "/order/details";
pub(super) const ORDERS_ORDER_HISTORY_ENDPOINT: &str = "/order/history";
pub(super) const ORDERS_ORDER_BOOK_ENDPOINT: &str = "/order/retrieve-all";
pub(super) const ORDERS_TRADES_ENDPOINT: &str = "/order/get-trades-for-day";
pub(super) const ORDERS_ORDER_TRADES_ENDPOINT: &str = "/order/trades";
pub(super) const ORDERS_TRADE_HISTORY_ENDPOINT: &str = "/charges/historical-trades";

pub(super) const GTT_ORDERS_PLACE_GTT_ORDER_ENDPOINT: &str = "order/gtt/place";
pub(super) const GTT_ORDERS_MODIFY_GTT_ORDER_ENDPOINT: &str = "order/gtt/modify";
pub(super) const GTT_ORDERS_CANCEL_GTT_ORDER_ENDPOINT: &str = "order/gtt/cancel";
pub(super) const GTT_ORDERS_GTT_ORDER_DETAILS_ENDPOINT: &str = "order/gtt";

pub(super) const TRADE_PNL_REPORT_METADATA_ENDPOINT: &str = "/trade/profit-loss/metadata";
pub(super) const TRADE_PNL_REPORT_ENDPOINT: &str = "/trade/profit-loss/data";
pub(super) const TRADE_PNL_TRADES_CHARGES_ENDPOINT: &str = "/trade/profit-loss/charges";

pub(super) const HISTORICAL_CANDLE_DATA_ENDPOINT: &str = "/historical-candle";
pub(super) const HISTORICAL_CANDLE_INTRADAY_DATA_ENDPOINT: &str = "/historical-candle/intraday";

pub(super) const PORTFOLIO_POSITIONS_ENDPOINT: &str = "/portfolio/short-term-positions";
pub(super) const PORTFOLIO_MTF_POSITIONS_ENDPOINT: &str = "/portfolio/mtf-positions";
pub(super) const PORTFOLIO_CONVERT_POSITIONS_ENDPOINT: &str = "/portfolio/convert-position";
pub(super) const PORTFOLIO_HOLDINGS_ENDPOINT: &str = "/portfolio/long-term-holdings";

pub(super) const MARKET_QUOTE_FULL_ENDPOINT: &str = "/market-quote/quotes";
pub(super) const MARKET_QUOTE_OHLC_ENDPOINT: &str = "/market-quote/ohlc";
pub(super) const MARKET_QUOTE_LTP_ENDPOINT: &str = "/market-quote/ltp";
pub(super) const MARKET_QUOTE_OPTION_GREEKS_ENDPOINT: &str = "/market-quote/option-greek";

pub(super) const MARKET_INFO_HOLIDAYS_ENDPOINT: &str = "/market/holidays";
pub(super) const MARKET_INFO_TIMINGS_ENDPOINT: &str = "/market/timings";
pub(super) const MARKET_INFO_EXCHANGE_STATUS_ENDPOINT: &str = "/market/status";

pub(super) const EXPIRED_INSTRUMENTS_EXPIRIES_ENDPOINT: &str = "/expired-instruments/expiries";
pub(super) const EXPIRED_INSTRUMENTS_EXPIRED_OPTION_CONTRACTS_ENDPOINT: &str =
    "/expired-instruments/option/contract";
pub(super) const EXPIRED_INSTRUMENTS_EXPIRED_FUTURE_CONTRACTS_ENDPOINT: &str =
    "/expired-instruments/future/contract";
pub(super) const EXPIRED_INSTRUMENTS_EXPIRED_HISTORICAL_CANDLE_DATA_ENDPOINT: &str =
    "/expired-instruments/historical-candle";

pub(super) const OPTION_CONTRACTS_ENDPOINT: &str = "/option/contract";
pub(super) const OPTION_CHAIN_ENDPOINT: &str = "/option/chain";

pub(super) const WS_PORTFOLIO_FEED_AUTHORIZE_ENDPOINT: &str =
    "/feed/portfolio-stream-feed/authorize";
pub(super) const WS_MARKET_DATA_FEED_AUTHORIZE_ENDPOINT: &str = "/feed/market-data-feed/authorize";

pub(super) const GOOGLE_IMAP_URL: &str = "imap.gmail.com";
pub(super) const GOOGLE_OAUTH2_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub(super) const GOOGLE_OAUTH2_ACCESS_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub(super) const GOOGLE_REFRESH_TOKEN_FILENAME: &str = "refresh_token.txt";
pub(super) const UPSTOX_ACCESS_TOKEN_FILENAME: &str = "access_token.txt";
pub(super) const INSTRUMENTS_ARCHIVE_FILENAME: &str = "complete.json.gz";
pub(super) const INSTRUMENTS_JSON_FILENAME: &str = "complete.json";

// ---------------------------------------------------------------------------
// Rate-limit bucket constants (Upstox Uplink REST).
//
// Source (verified 2026-04-20):
// https://upstox.com/developer/api-documentation/rate-limiting
//
// The legacy `RATE_LIMIT_PER_*` constants (pre-v2) are retained as
// `#[deprecated]` aliases below for one release so external users with
// custom `ApiRateLimiter::new(_, _, _)` calls still compile — the new
// `ApiRateLimiter::new(RateLimitProfile)` constructor consumes the new
// values and ignores the legacy triple entirely.
// ---------------------------------------------------------------------------

/// Combined order-placement bucket — per-second cap for non-SEBI-registered
/// algos. Applies to `/order/place`, `/order/modify`, `/order/cancel`,
/// `/order/multi/*`, `/order/positions/exit`, and `/order/gtt/{place,modify,cancel}`.
pub const ORDER_BUCKET_PER_SECOND_REGULAR: usize = 10;
/// Combined order-placement bucket — per-second cap for SEBI-registered algos.
pub const ORDER_BUCKET_PER_SECOND_SEBI: usize = 50;
/// Combined order-placement bucket — per-minute cap (both profiles).
pub const ORDER_BUCKET_PER_MINUTE: usize = 500;
/// Combined order-placement bucket — per-30-minute cap (both profiles).
pub const ORDER_BUCKET_PER_30_MINUTES: usize = 2000;

/// Standard read/account bucket — per-second cap.
pub const STANDARD_BUCKET_PER_SECOND: usize = 50;
/// Standard read/account bucket — per-minute cap.
pub const STANDARD_BUCKET_PER_MINUTE: usize = 500;
/// Standard read/account bucket — per-30-minute cap.
pub const STANDARD_BUCKET_PER_30_MINUTES: usize = 2000;

#[deprecated(note = "Use the bucket-specific constants (ORDER_BUCKET_* / STANDARD_BUCKET_*). The \
                     pre-v2 SDK modelled limits per-endpoint with a flat (25, 250, 1000) bucket \
                     which does not match Upstox's combined-bucket reality.")]
#[allow(dead_code)]
pub(super) const RATE_LIMIT_PER_SECOND: usize = 25;
#[deprecated(note = "Use the bucket-specific constants (ORDER_BUCKET_* / STANDARD_BUCKET_*).")]
#[allow(dead_code)]
pub(super) const RATE_LIMIT_PER_MINUTE: usize = 250;
#[deprecated(note = "Use the bucket-specific constants (ORDER_BUCKET_* / STANDARD_BUCKET_*).")]
#[allow(dead_code)]
pub(super) const RATE_LIMIT_PER_THIRTY_MINUTES: usize = 1000;

pub(super) const EMAIL_ID_ENV: &str = "EMAIL_ID";
pub(super) const GOOGLE_AUTHORIZATION_CODE_ENV: &str = "GOOGLE_AUTHORIZATION_CODE";
pub(super) const GOOGLE_CLIENT_ID_ENV: &str = "GOOGLE_CLIENT_ID";
pub(super) const GOOGLE_CLIENT_SECRET_ENV: &str = "GOOGLE_CLIENT_SECRET";
pub(super) const MOBILE_NUMBER_ENV: &str = "MOBILE_NUMBER";
pub(super) const LOGIN_PIN_ENV: &str = "LOGIN_PIN";
pub(super) const REDIRECT_PORT_ENV: &str = "REDIRECT_PORT";
pub const UPLINK_API_KEY_ENV: &str = "UPLINK_API_KEY";
pub(super) const UPLINK_API_SECRET_ENV: &str = "UPLINK_API_SECRET";
pub(super) const WEBDRIVER_SOCKET_ENV: &str = "WEBDRIVER_SOCKET";
