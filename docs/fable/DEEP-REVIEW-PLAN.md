# Deep review plan — upstox-rust-sdk fork

_Created 2026-07-14 (NOB session 18). Status: **planned, not
started**. Companion to the
[NOB-HFT deep review](https://gitlab.com/developers7143329/Nifty-Options-Buyer-HFT/-/blob/main/docs/fable/DEEP-REVIEW-PLAN.md)
— same 11-dimension checklist and working protocol apply; this file
adds the SDK-specific scope and one extra dimension._

## Why a full review

Upstream is unmaintained; we patch this fork manually against the
live Upstox API. Every correctness property a vendor would own —
endpoint paths, request/response schemas, rate-limit tables, WS
frame decoding, auth flows — is ours to verify. The fork sits
directly on the NOB-HFT money path (order placement, GTT amends,
market data), so an SDK bug is indistinguishable from a broker
incident at runtime.

## Extra review dimension (12): upstream/API parity

For every endpoint, model, and constant: does it match the CURRENT
Upstox API documentation (not the docs from when upstream froze)?
Record the doc URL + date checked in the tracker row. Fields that
Upstox added since the freeze must be either mapped or explicitly
tolerated (`deny_unknown_fields` posture decided consciously).
Special attention to:

- Rate-limit tables (order bucket 10/50 req/s Regular vs
  SEBI-registered; Plus WS slot counts) — NOB's
  `ClientCapabilities` contract depends on these.
- V3 order APIs (`market_protection`, `X-Algo-Name` header —
  required from 2026-04-01 for SEBI-registered algos).
- WS `full_d30` (30-level depth) framing + protobuf schema drift.
- Expired-instruments endpoints (Plus-gated).

## Phase plan

Priority: the NOB-HFT consumption surface first (what the gateway/
executor actually call), then the rest.

### Phase S0 — Contracts & meta (P0)

- `Cargo.toml`/`Cargo.lock` — dep audit; version vs upstream;
  edition/features.
- `build.rs` + `src/protos/` — protobuf codegen: which .proto
  version, does it match Upstox's current market-data feed spec.
- `src/constants.rs` — every URL/limit/magic number vs current API
  docs (dimension 12 sweep).
- `src/lib.rs` — public surface inventory: what NOB-HFT imports
  (cross-reference `ClientCapabilities`, `PlaceOrderV3Request`,
  `OptionContractsResponse`, subscribe modes) vs what is exported
  and unused (orphan candidates).

### Phase S1 — Money path (P0)

- `src/client.rs` — auth/token lifecycle, header injection
  (`X-Algo-Name`), error taxonomy, retry semantics (idempotency on
  order APIs — a blind retry on a timed-out place-order can double
  an entry).
- `src/rate_limiter.rs` — bucket definitions vs the current Upstox
  rate-limit table; Regular vs SEBI-registered vs Plus gating
  (`FeatureRequiresPlus`); burst vs sustained semantics; clock
  source.
- `src/apis/` order-placement bucket — Place/Modify/Cancel/Multi/
  ExitAll/GTT: request models field-by-field vs API docs,
  `market_protection` handling (-1/0/1..=25), response parsing on
  every error shape.

### Phase S2 — Market data path (P0)

- `src/ws_client.rs` — 5-WS layout, subscribe modes (ltpc / full /
  full_d30 / option_greeks), reconnect/backoff, frame decode errors
  (a silently dropped frame = a silent feature gap in NOB),
  Plus-gating refusals.
- `src/protos/` decode paths — field presence vs NOB's tick
  mappers (D30 depth 30 levels, greeks, LTPC).

### Phase S3 — Remaining REST surface (P1)

- `src/apis/` non-order endpoints — instruments, expiries, LTP
  quotes, historical, profile/funds. Verify the ones NOB's
  daily_refresh/startup_subscribe chain calls first.
- `src/models/` — serde attributes (rename/default/deny) per model;
  Option-ality matches API nullability; no silent-zero defaults on
  money fields.
- `src/utils/` — helpers; dead code sweep.

### Phase S4 — Meta & hygiene (P2)

- `examples/` — do they compile against the current fork? Stale
  examples mislead more than no examples.
- Root scripts (`create_issues.{ps1,py}`, `map_all_fields.py`,
  `map_project_items.py`, `project-setup-instructions.md`) —
  one-shot repo-automation artifacts: still needed? If not, delete
  (dimension 11); if kept, document their purpose in README.
- `.github/` — upstream CI leftovers vs the no-CI posture (NOB
  F17); `.vscode/` — personal editor config in a shared repo?
- `README.md`/`CHANGELOG.md` — does the README still describe
  upstream? It must state this is a maintained fork, what diverged,
  and the NOB consumption contract. CHANGELOG discipline for every
  manual API-sync.

### Phase S5 — Cross-repo synthesis (P1)

- Contract table: every symbol NOB-HFT imports → where defined →
  API doc reference → test coverage. This table is the fork's
  reason to exist; it also resolves NOB F23 (the consumption
  contract makes vendoring vs git-dependency an informed choice).
- Suppression audit (`#[allow]`/etc.) — same protocol as NOB
  session 12.
- File NOB-side issues for any behavioural mismatch found.

## Status tracker

| Phase | Scope | Status | Owner | Findings |
|---|---|---|---|---|
| S0 | contracts, protos, constants, lib surface | not-started | — | — |
| S1 | client, rate_limiter, order APIs | not-started | — | — |
| S2 | ws_client, proto decode | not-started | — | — |
| S3 | remaining apis, models, utils | not-started | — | — |
| S4 | examples, scripts, .github/.vscode, README/CHANGELOG | not-started | — | — |
| S5 | cross-repo contract table + suppressions | not-started | — | — |

## Exit criteria

- Every tracked file appears in a completed tracker row.
- Every endpoint/constant carries an API-doc-parity verdict with URL
  + date.
- The NOB consumption contract table exists and is green.
- Order-path retry/idempotency semantics documented and tested.
- Repo hygiene: no orphaned one-shot scripts, README states the
  fork's purpose, CHANGELOG covers every manual sync.
- Local gate green + NOB workspace builds against the reviewed fork.
