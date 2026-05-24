# Copilot / AI Agent Instructions — upstox-rust-sdk

> **Read this before any code change.** These instructions govern how AI coding agents operate in this repository.

## What this repository is

A Rust SDK for the [Upstox Uplink API](https://upstox.com/developer/api-documentation/open-api) — providing typed REST API bindings, WebSocket streaming (market data + portfolio feeds), OAuth2 login automation, and client-side rate limiting.

## Team

- **@OkBeiRohan** — ALWAYS REVIEWER for every PR. Final approver.
- **@goldr0g3r** — Primary assignee for all technical work.

## Hard rules — coding agents MUST follow

### 1. Conventional Commits
```
<type>(<scope>): <subject explaining WHY>
```
Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`.
Scopes: `client`, `rate-limiter`, `ws-client`, `login`, `orders`, `market-quote`, `historical-data`, `instruments`, `portfolio`, `charges`, `margins`, `gtt-orders`, `option-chain`, `market-info`, `trade-pnl`, `user`, `expired-instruments`, `models`, `utils`, `constants`, `protos`, `ci`, `deps`, `docs`.

### 2. No stubs in production code
- No `todo!()`, `unimplemented!()`, `panic!("not implemented")` in non-test code.
- No `// TODO: implement` without a linked issue number.
- Empty `match` arms that should do work = forbidden.

### 3. No warning suppression without justification
- No bare `#[allow(...)]` — must have a `// SAFETY:` or `// REASON:` comment + tracking issue.
- `cargo clippy -- -D warnings` must pass (CI enforces this).

### 4. Tests in same commit as code
- Every new public function gets a unit test in `#[cfg(test)] mod tests {}`.
- Use `#[tokio::test]` for async tests.
- Never hit real Upstox endpoints in tests — mock HTTP responses.
- Arrange-Act-Assert structure.

### 5. Documentation on every public item
- `///` doc comments on all `pub fn`, `pub struct`, `pub enum`, `pub trait`.
- Module-level `//!` documentation explaining the module's purpose.

### 6. Error handling
- Use `Result<T, E>` — never `unwrap()` in production code without a `// SAFETY:` comment.
- `panic!` only for invariant violations that indicate programmer error.
- Prefer custom error types over `String` errors.

### 7. Before every PR
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build
```
All four must pass. CI runs these automatically.

### 8. Commit discipline
- Subject ≤ 72 chars, imperative mood.
- One logical change per commit.
- Never commit secrets (API keys, tokens, `.env` files).
- Never `--force-push` to `main`.
- Never include `Co-Authored-By: <agent>` trailers.

## Module map

```
src/
├── lib.rs           # Crate root + re-exports
├── client.rs        # ApiClient — construction, HTTP dispatch, scheduling
├── rate_limiter.rs   # Dual-bucket sliding-window rate limiter
├── ws_client.rs      # WebSocket pool (market data + portfolio feeds)
├── constants.rs      # Endpoints, rate-limit caps, env var names
├── apis/            # REST API implementations (one file per Upstox API group)
├── models/          # Request/response types (serde-driven)
├── protos/          # Protobuf definitions for market data feed
└── utils/           # URL builder, serde helpers
```

## PR workflow

1. Apply correct labels: `type/*` + `area/*` + `priority/*`.
2. Fill the PR template completely (Summary, Type, Scope, checklist).
3. The auto-assign workflow routes assignee + reviewer automatically.
4. The merge-on-approval workflow squash-merges once @OkBeiRohan approves + CI green.
