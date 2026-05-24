$ErrorActionPreference = "Stop"

Write-Host "Creating Epic Issue..."
$epicOutput = gh issue create --repo OkBeiRohan/upstox-rust-sdk --title "[EPIC] Full Codebase Review — upstox-rust-sdk v2.0.0" --label "type/code-review,priority/P1-high" --body "# Full Codebase Review — upstox-rust-sdk v2.0.0

## Objective
Systematic review of all ~6,200 lines across 108 .rs source files. Identify bugs, improve error handling, add missing unit tests, enforce Rust best practices, and improve documentation.

## Current Test Coverage
Only 5 files have unit tests:
- \`src/client.rs\` — ClientCapabilities tests
- \`src/rate_limiter.rs\` — classify_endpoint + bucket tests
- \`src/models/mod.rs\` — Exchange round-trip tests
- \`src/models/ws/market_data_feed_v3_message.rs\` — ModeTypeV3 tests
- \`src/ws_client.rs\` — WsConnectionId/Role tests

Most API, model, and utility modules have ZERO tests.

## Review Checklist per Module
- [ ] No \`unwrap()\` in production paths without safety comment
- [ ] No \`panic!()\` / \`todo!()\` / \`unimplemented!()\`
- [ ] Error handling uses \`Result<T, E>\` properly
- [ ] \`///\` doc comments on all public items
- [ ] Unit tests in \`#[cfg(test)] mod tests {}\`
- [ ] Serde attributes correct
- [ ] Proper ownership/borrowing

## Sub-Issues
See linked issues below — each covers one module or file group.

## PR Workflow for Review Fixes
1. Branch: \`review/<module-name>\`
2. Commit: \`refactor(<scope>): <what and why>\` or \`test(<scope>): add unit tests\`
3. Reference the sub-issue: \`Closes #N\`
4. Ensure \`cargo test && cargo clippy -- -D warnings && cargo fmt --check\` passes
5. PR auto-assigns @goldr0g3r, requests @OkBeiRohan review
"

# Extract epic number from URL like https://github.com/OkBeiRohan/upstox-rust-sdk/issues/1
$epicNum = $epicOutput.Split("/")[-1]
Write-Host "Created Epic #$epicNum"

function Create-Issue($Title, $Labels, $Body) {
    $BodyWithEpic = $Body.Replace("EPIC_NUM", $epicNum)
    $output = gh issue create --repo OkBeiRohan/upstox-rust-sdk --title $Title --label $Labels --body $BodyWithEpic
    Write-Host "Created: $output"
    Start-Sleep -Seconds 2
}

Create-Issue -Title "review: client.rs — ApiClient construction & HTTP request dispatch" -Labels "type/code-review,priority/P0-critical,area/client" -Body "# Review: \`src/client.rs\` (1,126 lines)

Part of #EPIC_NUM

## Files
| File | Lines | Has Tests? |
|------|-------|------------|
| \`src/client.rs\` | 1,126 | Yes (5 tests on ClientCapabilities only) |

## Review Focus Areas

### 1. \`ApiClient::new_with_capabilities\` (L554-L745)
- [ ] Verify scheduler lifecycle — \`shutdown_on_ctrl_c()\` may interfere with library consumers
- [ ] Check lock ordering — nested \`shared_api_client.lock().await\` inside constructor
- [ ] Review staggered WS connection logic — \`PER_SLOT_OPEN_BUDGET\`, \`INTER_CONNECT_DELAY\`
- [ ] Duplicate slot detection (L653-L671) — verify correctness

### 2. HTTP dispatch (\`request\` method, L843-L927)
- [ ] L870: \`authorized && !self.token.is_some()\` — uses \`panic!\` — should return \`Err\`
- [ ] L880: \`method.clone()\` — unnecessary clone, \`Method\` is \`Copy\`-like
- [ ] L901: \`self.token.as_ref().unwrap()\` — safe due to L870 check but fragile
- [ ] L910-L917: X-Algo-Name header injection — verify scope is correct

### 3. Scheduling (L980-L1024)
- [ ] \`schedule_refresh_instruments\` — cron expression \`0 30 06 * * *\` hardcoded
- [ ] \`schedule_auto_login\` — \`.unwrap()\` on login failure inside scheduled job

### 4. Config types (L1027-L1070)
- [ ] \`LoginConfig\` — \`Clone\` but contains \`Option<AutomateLoginConfig>\`
- [ ] \`WSConnectConfig\` — not \`Clone\`/\`Debug\` due to callback boxes
- [ ] \`WsChannelConfig\` — not \`Debug\` due to callback box

## Unit Test Plan

### Tests to ADD:
1. **\`request\` method error paths**: Mock reqwest to verify \`RateLimitExceeded::Network\` on transport failure
2. **Unauthorized panic**: Verify panics (or better: refactor to \`Result\`) when \`authorized=true\` + no token
3. **\`set_algo_name\` / \`algo_name\`**: Round-trip test
4. **\`ensure_plus_user\`**: Returns \`Ok\` for plus, \`Err\` for non-plus
5. **\`verify_authorization\`**: Mock profile response, verify true/false

## PR Instructions
1. Branch: \`review/client\`
2. Closes this issue"

Create-Issue -Title "review: rate_limiter.rs — dual-bucket sliding-window rate limiter" -Labels "type/code-review,priority/P0-critical,area/rate-limiter" -Body "# Review: \`src/rate_limiter.rs\` (526 lines)

Part of #EPIC_NUM

## Review Focus Areas
### 1. \`RateLimitExceeded\` enum (L47-L74)
- [ ] \`#[non_exhaustive]\` is good — verify all match arms handle \`_\` wildcard
- [ ] \`Network(String)\` — should this carry the original \`reqwest::Error\` instead?
- [ ] No \`std::error::Error\` impl — should implement for ergonomic \`?\` usage

### 2. \`RateLimiter::check_rate_limit\` (L157-L199)
- [ ] L161: \`retain\` scans entire VecDeque on every call — O(n) where n can be 2000
- [ ] L163-L166: per-second filter rescans after retain — double iteration
- [ ] Thread safety: \`Mutex<VecDeque>\` — is \`tokio::sync::Mutex\` needed or would \`std::sync::Mutex\` suffice?

### 3. \`classify_endpoint\` (L112-L136)
- [ ] Linear scan of 9 prefixes — fine for now but could be a const array match
- [ ] GTT paths don't have leading \`/\` — handled by \`trim_start_matches\` but fragile

### 4. \`acquire_slot\` (L301-L321)
- [ ] Infinite loop with 10ms jitter — could this starve under extreme contention?

## Unit Test Plan
1. \`RateLimitExceeded\` Display/Debug
2. \`classify_endpoint\` edge cases
3. Per-minute cap triggering
4. Concurrent \`acquire_slot\`

## PR Instructions
1. Branch: \`review/rate-limiter\`
2. Closes this issue"

Create-Issue -Title "review: ws_client.rs — WebSocket pool, reconnect, protobuf decode" -Labels "type/code-review,priority/P0-critical,area/ws-client" -Body "# Review: \`src/ws_client.rs\` (800 lines)

Part of #EPIC_NUM

## Review Focus Areas
### 1. Connection pool types
- [ ] \`MarketDataFeedV3ClientPool\` sizing
- [ ] \`WsConnectionId\` bounds checking

### 2. WebSocket lifecycle
- [ ] Reconnection logic — supervisor watchdog cycle
- [ ] Error handling on WS disconnect — does it panic or recover?

### 3. Portfolio feed client
- [ ] JSON deserialization of portfolio updates
- [ ] Callback invocation safety

### 4. Thread safety
- [ ] \`Send + Sync\` bounds on callback boxes

## Unit Test Plan
1. WsConnectionId bounds
2. Protobuf decode
3. Gzip decompression
4. Error variants

## PR Instructions
1. Branch: \`review/ws-client\`
2. Closes this issue"

Create-Issue -Title "review: constants.rs — API endpoints, rate-limit caps, env vars" -Labels "type/code-review,priority/P1-high,area/client" -Body "# Review: \`src/constants.rs\` (146 lines)

Part of #EPIC_NUM

## Review Focus Areas
- [ ] Verify all endpoint paths match current Upstox API docs
- [ ] Check deprecated constants have clear migration paths
- [ ] Verify rate-limit values match Upstox docs
- [ ] Check env var naming consistency
- [ ] Visibility: most are \`pub(super)\` — is this correct?

## Unit Test Plan
1. Endpoint completeness
2. Rate limit values
3. Env var names

## PR Instructions
1. Branch: \`review/constants\`
2. Closes this issue"

Create-Issue -Title "review: lib.rs + build.rs — crate root, re-exports, protobuf codegen" -Labels "type/code-review,priority/P2-medium,area/infra" -Body "# Review: \`src/lib.rs\` + \`build.rs\` (44 lines total)

Part of #EPIC_NUM

## Review Focus Areas
- [ ] \`lib.rs\`: Re-exports complete?
- [ ] \`lib.rs\`: Module visibility
- [ ] \`build.rs\`: Protobuf codegen — verify output path, error handling
- [ ] Crate-level docs (\`//!\`)

## PR Instructions
1. Branch: \`review/crate-root\`
2. Closes this issue"

Create-Issue -Title "review: apis/login.rs — OAuth, OTP automation, webdriver, token management" -Labels "type/code-review,priority/P0-critical,area/login" -Body "# Review: \`src/apis/login.rs\` (525 lines)

Part of #EPIC_NUM

## Review Focus Areas
### 1. OAuth2 flow
- [ ] Token exchange params
- [ ] Access token file persistence security

### 2. OTP automation (SECURITY CRITICAL)
- [ ] Gmail IMAP access credentials
- [ ] OTP extraction regex reliability

### 3. Error handling
- [ ] How many \`unwrap()\` calls?
- [ ] Network failure handling during login

### 4. Secret handling
- [ ] Are API keys/secrets logged anywhere?

## Unit Test Plan
1. Token request construction
2. OTP regex extraction
3. Access token file read/write

## PR Instructions
1. Branch: \`review/login\`
2. Closes this issue"

Create-Issue -Title "review: apis/orders.rs — place/modify/cancel/multi orders, trade history" -Labels "type/code-review,priority/P0-critical,area/orders" -Body "# Review: \`src/apis/orders.rs\` (402 lines)

Part of #EPIC_NUM

## Review Focus Areas
- [ ] Order placement request validation
- [ ] Multi-order placement batch handling
- [ ] Cancel order error handling
- [ ] Modify order partial update handling
- [ ] \`unwrap()\` usage

## Unit Test Plan
1. Place order request serialization
2. Multi-order request validation
3. Trade history date parsing

## PR Instructions
1. Branch: \`review/orders\`
2. Closes this issue"

Create-Issue -Title "review: apis/market_quote.rs — full/OHLC/LTP/option greeks quotes" -Labels "type/code-review,priority/P1-high,area/market-data" -Body "# Review: \`src/apis/market_quote.rs\` (181 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] V2 vs V3 API differentiation
- [ ] Query param construction
- [ ] Error handling on API failures

## Unit Test Plan
1. Query param serialization
2. Response deserialization
3. V3 endpoint routing

## PR Instructions
1. Branch: \`review/market-quote\`
2. Closes this issue"

Create-Issue -Title "review: apis/historical_data.rs — candle data, intraday, V3 params" -Labels "type/code-review,priority/P1-high,area/market-data" -Body "# Review: \`src/apis/historical_data.rs\` (154 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] Date range param formatting
- [ ] Interval enum handling
- [ ] V3 request differences

## Unit Test Plan
1. URL construction
2. V3 request param serialization

## PR Instructions
1. Branch: \`review/historical-data\`
2. Closes this issue"

Create-Issue -Title "review: apis/instruments.rs — instrument archive fetch & parse" -Labels "type/code-review,priority/P1-high,area/market-data" -Body "# Review: \`src/apis/instruments.rs\` (161 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] Gzip download and decompression
- [ ] JSON parsing of large archive
- [ ] HashMap grouping
- [ ] File caching logic race safety

## Unit Test Plan
1. \`parse_instruments\`
2. Gzip decompression
3. Edge cases

## PR Instructions
1. Branch: \`review/instruments\`
2. Closes this issue"

Create-Issue -Title "review: apis/expired_instruments.rs — Plus-only expired contracts" -Labels "type/code-review,priority/P2-medium,area/market-data" -Body "# Review: \`src/apis/expired_instruments.rs\` (135 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] \`ensure_plus_user\` guard
- [ ] Request param construction

## Unit Test Plan
1. Plus-only guard
2. Request param serialization

## PR Instructions
1. Branch: \`review/expired-instruments\`
2. Closes this issue"

Create-Issue -Title "review: apis/gtt_orders.rs — GTT place/modify/cancel/details" -Labels "type/code-review,priority/P1-high,area/orders" -Body "# Review: \`src/apis/gtt_orders.rs\` (122 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] GTT endpoint paths
- [ ] Request validation

## PR Instructions
1. Branch: \`review/gtt-orders\`
2. Closes this issue"

Create-Issue -Title "review: apis/portfolio.rs — positions, holdings, convert" -Labels "type/code-review,priority/P1-high,area/portfolio" -Body "# Review: \`src/apis/portfolio.rs\` (110 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] Convert position request validation
- [ ] Holdings response parsing

## PR Instructions
1. Branch: \`review/portfolio\`
2. Closes this issue"

Create-Issue -Title "review: apis/market_information.rs — holidays, timings, exchange status" -Labels "type/code-review,priority/P2-medium,area/market-data" -Body "# Review: \`src/apis/market_information.rs\` (109 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] Holiday/timing query params
- [ ] Date formatting

## PR Instructions
1. Branch: \`review/market-info\`
2. Closes this issue"

Create-Issue -Title "review: apis/user.rs — profile, funds & margin (V2 + V3)" -Labels "type/code-review,priority/P1-high,area/client" -Body "# Review: \`src/apis/user.rs\` (96 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] V2 vs V3 endpoint routing
- [ ] Profile response parsing

## PR Instructions
1. Branch: \`review/user\`
2. Closes this issue"

Create-Issue -Title "review: apis/trade_profit_and_loss.rs — PnL reports & trades charges" -Labels "type/code-review,priority/P2-medium,area/portfolio" -Body "# Review: \`src/apis/trade_profit_and_loss.rs\` (95 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] Date range param formatting
- [ ] PnL report metadata vs data endpoints

## PR Instructions
1. Branch: \`review/trade-pnl\`
2. Closes this issue"

Create-Issue -Title "review: apis/option_chain.rs + apis/charges.rs + apis/margins.rs" -Labels "type/code-review,priority/P2-medium,area/market-data" -Body "# Review: Small API modules (154 lines total)

Part of #EPIC_NUM

## Review Focus
- [ ] Request param construction
- [ ] Response type mapping

## PR Instructions
1. Branch: \`review/small-apis\`
2. Closes this issue"

Create-Issue -Title "review: models/mod.rs — shared enums (Exchange, OrderStatus, AssetType, etc.)" -Labels "type/code-review,priority/P0-critical,area/models" -Body "# Review: \`src/models/mod.rs\` (412 lines)

Part of #EPIC_NUM

## Review Focus
### 1. Permissive enums (Exchange, ExchangeSegment, AssetType)
- [ ] \`Other(String)\` variant
- [ ] \`From<String>\` impls

### 2. OrderStatus (L280-L370)
- [ ] Custom serde via \`serde_spaced_lowercase\`
- [ ] \`FromStr\` error handling

### 3. ProductType / TransactionType / SegmentType / OrderVariety
- [ ] \`rename_all = UPPERCASE\`

## Unit Test Plan
1. ExchangeSegment round-trip
2. AssetType round-trip
3. OrderStatus serde round-trip

## PR Instructions
1. Branch: \`review/models-core\`
2. Closes this issue"

Create-Issue -Title "review: models/orders/ — all order request/response types" -Labels "type/code-review,priority/P1-high,area/orders" -Body "# Review: \`src/models/orders/\` (~290 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] \`serde_valid\` validation rules
- [ ] \`market_protection\` field validator
- [ ] V3 order request differences

## PR Instructions
1. Branch: \`review/models-orders\`
2. Closes this issue"

Create-Issue -Title "review: models/ws/ — WebSocket message types" -Labels "type/code-review,priority/P1-high,area/ws-client" -Body "# Review: \`src/models/ws/\` (~248 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] ModeTypeV3 — Plus-only modes
- [ ] PortfolioFeedResponse deserialization

## PR Instructions
1. Branch: \`review/models-ws\`
2. Closes this issue"

Create-Issue -Title "review: models/market_quote/ — quote request/response types" -Labels "type/code-review,priority/P2-medium,area/market-data" -Body "# Review: \`src/models/market_quote/\` (~151 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] V2 vs V3 response type differences
- [ ] Interval enum serialization

## PR Instructions
1. Branch: \`review/models-market-quote\`
2. Closes this issue"

Create-Issue -Title "review: models/historical_data/ + models/instruments/" -Labels "type/code-review,priority/P2-medium,area/market-data" -Body "# Review: Historical data + instruments models (~170 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] HistoricalInterval enum
- [ ] InstrumentsResponse field types

## PR Instructions
1. Branch: \`review/models-historical\`
2. Closes this issue"

Create-Issue -Title "review: models/user/ + models/login/ + models/portfolio/" -Labels "type/code-review,priority/P2-medium,area/models" -Body "# Review: User, login, portfolio models (~200 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] V3 fund/margin response
- [ ] Google OAuth2 request types
- [ ] Position response precision

## PR Instructions
1. Branch: \`review/models-user-login-portfolio\`
2. Closes this issue"

Create-Issue -Title "review: models/gtt_orders/ + models/option_chain/ + models/charges/ + models/margins/ + models/trade_pnl/ + models/expired_instruments/" -Labels "type/code-review,priority/P2-medium,area/models" -Body "# Review: Remaining model modules (~400 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] GTT order type enums
- [ ] Option chain response nested structure
- [ ] BrokerageDetailsRequest validation

## PR Instructions
1. Branch: \`review/models-remaining\`
2. Closes this issue"

Create-Issue -Title "review: models/error_response.rs + models/success_response.rs" -Labels "type/code-review,priority/P1-high,area/models" -Body "# Review: Error/Success response wrappers (~45 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] \`ErrorResponse\` fields completeness
- [ ] \`SuccessResponse<T>\` generics

## PR Instructions
1. Branch: \`review/response-types\`
2. Closes this issue"

Create-Issue -Title "review: utils/mod.rs + utils/serde_spaced_lowercase.rs" -Labels "type/code-review,priority/P2-medium,area/client" -Body "# Review: Utility modules (87 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] \`create_url\` URL construction
- [ ] Base URL constants
- [ ] \`serde_spaced_lowercase\`

## PR Instructions
1. Branch: \`review/utils\`
2. Closes this issue"

Create-Issue -Title "review: protos/ — protobuf definition & codegen" -Labels "type/code-review,priority/P3-low,area/infra" -Body "# Review: Protobuf files (56 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] Proto message definitions
- [ ] Field numbers stable

## PR Instructions
1. Branch: \`review/protos\`
2. Closes this issue"

Create-Issue -Title "review: Cargo.toml — dependencies, features, metadata" -Labels "type/code-review,priority/P1-high,area/infra" -Body "# Review: \`Cargo.toml\` (39 lines)

Part of #EPIC_NUM

## Review Focus
- [ ] Dependencies needed?
- [ ] Version pinning strategy
- [ ] \`[dev-dependencies]\` missing

## Action Items
1. Update \`repository\` URL
2. Add \`[dev-dependencies]\`
3. Add \`homepage\` and \`documentation\`

## PR Instructions
1. Branch: \`review/cargo\`
2. Closes this issue"

Create-Issue -Title "review: examples/ — all 5 example binaries" -Labels "type/code-review,priority/P3-low,area/infra" -Body "# Review: Example binaries

Part of #EPIC_NUM

## Review Focus
- [ ] Compile with current API?
- [ ] Documented with comments?
- [ ] Error handling patterns

## PR Instructions
1. Branch: \`review/examples\`
2. Closes this issue"

Write-Host "All issues created successfully!"
