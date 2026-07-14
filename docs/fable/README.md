# docs/fable — collaboration workspace (upstox-rust-sdk fork)

Shared working folder for review findings, issue tracking, and
session hand-offs — same conventions as the
[Nifty-Options-Buyer-HFT workspace](https://gitlab.com/developers7143329/Nifty-Options-Buyer-HFT/-/tree/main/docs/fable).
Treat it as the fork's operational memory.

**Why this fork exists:** the upstream `upstox-rust-sdk` is not
maintained; we update it manually against the live Upstox API. That
makes US responsible for correctness normally delegated to a vendor
— hence the standing review plan below. The consumer is the
NOB-HFT workspace via `[patch.crates-io]` (see NOB issue F23).

| File | What it is | Update when |
|---|---|---|
| [DEEP-REVIEW-PLAN.md](DEEP-REVIEW-PLAN.md) | Full line-level review plan for the fork — phases, checklist, tracker | A phase starts/finishes or scope changes |
| [ISSUES.md](ISSUES.md) | Issue registry — U-items (SDK code), UD-items (doc rot). One row per issue, never renumber | You find, claim, or fix an issue |
| [PROGRESS.md](PROGRESS.md) | Done / in-progress / next-up board | An issue changes state |
| [SESSION-LOG.md](SESSION-LOG.md) | Append-only per-session narrative | Every working session, before you stop |

## Working agreement

1. Read PROGRESS.md → ISSUES.md before starting; claim a row before
   touching it.
2. The merge gate is the local gate: `cargo fmt --check` ·
   `cargo clippy --all-targets -- -D warnings` · `cargo test` — AND
   a NOB-HFT workspace build against the changed fork (the only
   real consumer is the integration test).
3. Log every session in SESSION-LOG.md (newest on top).
4. Any behaviour change that affects NOB-HFT (rate limits, WS
   layout, capabilities, response models) gets a cross-link to the
   NOB docs/fable ISSUES row it touches.
