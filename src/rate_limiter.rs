//! Rate-limiter for the Upstox Uplink REST API.
//!
//! Models the two distinct limit buckets the Upstox docs call out:
//!
//! - **Order placement** — shared across `order/place`, `order/modify`,
//!   `order/cancel`, `order/multi/*`, `order/positions/exit`, and
//!   `order/gtt/{place,modify,cancel}`. The per-second cap differs
//!   between non-SEBI-registered algos (10/s) and SEBI-registered
//!   ones (50/s).
//! - **Standard** — everything else (holdings, positions, funds,
//!   candles, quotes, option chain, market info, charges, user,
//!   logout, …). Uniform 50/s · 500/min · 2000/30min.
//!
//! Reference: <https://upstox.com/developer/api-documentation/rate-limiting>
//!
//! Pre-v2 SDK releases created a fresh `(25, 250, 1000)` bucket per
//! endpoint URL, which was both wrong shape (Upstox limits are
//! combined across endpoints, not siloed) and wrong numbers. The v2
//! rewrite preserves the `check_rate_limit(endpoint)` entry-point so
//! existing call sites keep compiling, but the internal routing goes
//! through [`classify_endpoint`] → one of [`RateLimitBucket::OrderPlacement`]
//! / [`RateLimitBucket::Standard`].

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

use crate::constants::{
    ORDER_BUCKET_PER_30_MINUTES, ORDER_BUCKET_PER_MINUTE, ORDER_BUCKET_PER_SECOND_REGULAR,
    ORDER_BUCKET_PER_SECOND_SEBI, STANDARD_BUCKET_PER_30_MINUTES, STANDARD_BUCKET_PER_MINUTE,
    STANDARD_BUCKET_PER_SECOND,
};

/// Outer error returned by every `ApiClient` method.
///
/// Name is historical — originally modelled rate-limit failures only;
/// v2 broadens it into a general SDK error envelope. The `Network` /
/// `UnsupportedMethod` / `FeatureRequiresPlus` variants replace the
/// earlier `.unwrap()` / `panic!` paths inside `ApiClient::request`
/// and `ApiClient::connect_market_data_feed_v3` so transient reqwest
/// errors no longer abort the process and non-Plus users get a clear
/// refusal instead of a silent exchange rejection. Callers that
/// exhaustively matched the three rate-limit variants need to add a
/// wildcard arm (`#[non_exhaustive]` enforces it at compile time).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RateLimitExceeded {
    PerSecond { next_allowed_at: Instant },
    PerMinute { next_allowed_at: Instant },
    PerThirtyMinutes { next_allowed_at: Instant },
    /// Transport-level failure forwarding the request — reqwest error,
    /// DNS miss, TLS handshake abort, or socket-level timeout. The
    /// inner string is the reqwest error's `Display` for operator
    /// logging.
    Network(String),
    /// The SDK was asked to issue an HTTP method it does not route
    /// (anything other than GET / POST / PUT / DELETE). The inner
    /// string is the method name for logging.
    UnsupportedMethod(String),
    /// Caller invoked a feature that requires an Upstox Plus account
    /// (more than 2 market-data WebSocket connections, `full_d30`
    /// mode subscribes, or expired-instruments REST endpoints) without
    /// setting [`crate::client::ClientCapabilities::is_plus_user`].
    /// The inner string names the feature for operator logging.
    FeatureRequiresPlus(String),
}

/// Which shared bucket an endpoint belongs to under the Upstox limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitBucket {
    /// Combined `order/place`, `order/modify`, `order/cancel`,
    /// `order/multi/*`, `order/positions/exit`, `order/gtt/{place,modify,cancel}`.
    OrderPlacement,
    /// Everything else — quotes, candles, holdings, positions, funds,
    /// charges, user, market info, option chain, login/logout.
    Standard,
}

/// Algorithm-registration tier of the client. Only affects the
/// per-second cap on the `OrderPlacement` bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitProfile {
    /// Non-registered algo trading (≤10 orders/s hard cap).
    RegularAlgo,
    /// SEBI-registered algo (bumped to 50/s on the order bucket).
    SebiRegistered,
}

impl RateLimitProfile {
    const fn order_bucket_per_second(self) -> usize {
        match self {
            RateLimitProfile::RegularAlgo => ORDER_BUCKET_PER_SECOND_REGULAR,
            RateLimitProfile::SebiRegistered => ORDER_BUCKET_PER_SECOND_SEBI,
        }
    }
}

/// Classify an endpoint path into its rate-limit bucket.
///
/// Matches on the bare endpoint string (e.g. `"/order/place"`, `"order/gtt/modify"`).
/// Kept `pub` so the algo-name header injector in `client.rs` can reuse it
/// — header is attached to order-bucket endpoints only per the 2026-04-01
/// SEBI circular.
pub fn classify_endpoint(endpoint: &str) -> RateLimitBucket {
    // Strip a leading slash for uniform matching (constants.rs is split —
    // most start with `/`, GTT paths do not).
    let e = endpoint.trim_start_matches('/');
    // NB: order matters — longer prefixes checked first so
    // `order/gtt/place` is classified as `OrderPlacement` rather than
    // falling through.
    const ORDER_PLACEMENT_PREFIXES: &[&str] = &[
        "order/place",
        "order/multi/place",
        "order/modify",
        "order/cancel",
        "order/multi/cancel",
        "order/positions/exit",
        "order/gtt/place",
        "order/gtt/modify",
        "order/gtt/cancel",
    ];
    for prefix in ORDER_PLACEMENT_PREFIXES {
        if e.starts_with(prefix) {
            return RateLimitBucket::OrderPlacement;
        }
    }
    RateLimitBucket::Standard
}

/// Single-bucket sliding-window limiter.
#[derive(Debug)]
struct RateLimiter {
    per_second: usize,
    per_minute: usize,
    per_thirty_minutes: usize,
    requests: Mutex<VecDeque<Instant>>,
}

impl RateLimiter {
    fn new(per_second: usize, per_minute: usize, per_thirty_minutes: usize) -> Self {
        Self {
            per_second,
            per_minute,
            per_thirty_minutes,
            requests: Mutex::new(VecDeque::new()),
        }
    }

    async fn check_rate_limit(&self) -> Option<RateLimitExceeded> {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        requests.retain(|&t| now.duration_since(t) <= Duration::from_secs(1800));

        let per_second_count = requests
            .iter()
            .filter(|&&t| now.duration_since(t) < Duration::from_secs(1))
            .count();
        if per_second_count >= self.per_second {
            let next_allowed_at = requests
                .iter()
                .find(|&&t| now.duration_since(t) < Duration::from_secs(1))
                .map(|&t| t + Duration::from_secs(1))
                .unwrap_or(now + Duration::from_secs(1));
            return Some(RateLimitExceeded::PerSecond { next_allowed_at });
        }

        let per_minute_count = requests
            .iter()
            .filter(|&&t| now.duration_since(t) < Duration::from_secs(60))
            .count();
        if per_minute_count >= self.per_minute {
            let next_allowed_at = requests
                .iter()
                .find(|&&t| now.duration_since(t) < Duration::from_secs(60))
                .map(|&t| t + Duration::from_secs(60))
                .unwrap_or(now + Duration::from_secs(60));
            return Some(RateLimitExceeded::PerMinute { next_allowed_at });
        }

        if requests.len() >= self.per_thirty_minutes {
            let next_allowed_at = requests
                .front()
                .map(|&t| t + Duration::from_secs(1800))
                .unwrap_or(now + Duration::from_secs(1800));
            return Some(RateLimitExceeded::PerThirtyMinutes { next_allowed_at });
        }

        requests.push_back(now);
        None
    }
}

/// Two-bucket rate limiter fronting the Upstox API.
///
/// Holds exactly one [`RateLimiter`] per [`RateLimitBucket`]; the old
/// per-endpoint map is gone because Upstox limits are combined, not
/// siloed. Safe to clone handles via `Arc` since the internal state
/// lives behind `Mutex`.
#[derive(Debug)]
pub struct ApiRateLimiter {
    order_bucket: Arc<RateLimiter>,
    standard_bucket: Arc<RateLimiter>,
    profile: RateLimitProfile,
}

impl ApiRateLimiter {
    pub fn new(profile: RateLimitProfile) -> Self {
        Self {
            order_bucket: Arc::new(RateLimiter::new(
                profile.order_bucket_per_second(),
                ORDER_BUCKET_PER_MINUTE,
                ORDER_BUCKET_PER_30_MINUTES,
            )),
            standard_bucket: Arc::new(RateLimiter::new(
                STANDARD_BUCKET_PER_SECOND,
                STANDARD_BUCKET_PER_MINUTE,
                STANDARD_BUCKET_PER_30_MINUTES,
            )),
            profile,
        }
    }

    pub fn profile(&self) -> RateLimitProfile {
        self.profile
    }

    pub async fn check_rate_limit(&self, endpoint: &str) -> Option<RateLimitExceeded> {
        let bucket = match classify_endpoint(endpoint) {
            RateLimitBucket::OrderPlacement => &self.order_bucket,
            RateLimitBucket::Standard => &self.standard_bucket,
        };
        bucket.check_rate_limit().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_routes_order_paths_to_order_bucket() {
        for path in [
            "/order/place",
            "/order/modify",
            "/order/cancel",
            "/order/multi/place",
            "/order/multi/cancel",
            "/order/positions/exit",
            "order/gtt/place",
            "order/gtt/modify",
            "order/gtt/cancel",
        ] {
            assert_eq!(
                classify_endpoint(path),
                RateLimitBucket::OrderPlacement,
                "expected {path} in OrderPlacement bucket"
            );
        }
    }

    #[test]
    fn classify_routes_reads_and_accounts_to_standard_bucket() {
        for path in [
            "/market-quote/ltp",
            "/market-quote/ohlc",
            "/market-quote/quotes",
            "/market-quote/option-greek",
            "/market/holidays",
            "/market/timings",
            "/order/retrieve-all",
            "/order/details",
            "/order/history",
            "order/gtt", // GTT details is a READ — Standard bucket
            "/portfolio/long-term-holdings",
            "/portfolio/short-term-positions",
            "/user/profile",
            "/user/get-funds-and-margin",
            "/charges/brokerage",
        ] {
            assert_eq!(
                classify_endpoint(path),
                RateLimitBucket::Standard,
                "expected {path} in Standard bucket"
            );
        }
    }

    #[tokio::test]
    async fn order_bucket_triggers_per_second_on_regular_profile() {
        let rl = ApiRateLimiter::new(RateLimitProfile::RegularAlgo);
        // 10 OK, 11th should hit per-second.
        for _ in 0..ORDER_BUCKET_PER_SECOND_REGULAR {
            assert!(rl.check_rate_limit("/order/place").await.is_none());
        }
        let exceeded = rl.check_rate_limit("/order/place").await;
        assert!(matches!(exceeded, Some(RateLimitExceeded::PerSecond { .. })));
    }

    #[tokio::test]
    async fn order_bucket_allows_more_on_sebi_profile() {
        let rl = ApiRateLimiter::new(RateLimitProfile::SebiRegistered);
        // 12 placements in one tick must not trip the per-second cap
        // (SEBI limit is 50/s).
        for _ in 0..12 {
            assert!(rl.check_rate_limit("/order/place").await.is_none());
        }
    }

    #[tokio::test]
    async fn order_and_modify_share_the_same_bucket() {
        let rl = ApiRateLimiter::new(RateLimitProfile::RegularAlgo);
        // 5 places + 5 modifies = 10 order-bucket hits. 11th (either
        // side) must trip per-second because the bucket is shared.
        for _ in 0..5 {
            assert!(rl.check_rate_limit("/order/place").await.is_none());
        }
        for _ in 0..5 {
            assert!(rl.check_rate_limit("/order/modify").await.is_none());
        }
        assert!(matches!(
            rl.check_rate_limit("/order/cancel").await,
            Some(RateLimitExceeded::PerSecond { .. })
        ));
    }

    #[tokio::test]
    async fn standard_and_order_buckets_are_independent() {
        let rl = ApiRateLimiter::new(RateLimitProfile::RegularAlgo);
        // Exhaust the order bucket.
        for _ in 0..ORDER_BUCKET_PER_SECOND_REGULAR {
            assert!(rl.check_rate_limit("/order/place").await.is_none());
        }
        // The standard bucket must NOT be affected.
        assert!(rl
            .check_rate_limit("/market-quote/ltp")
            .await
            .is_none());
    }
}
