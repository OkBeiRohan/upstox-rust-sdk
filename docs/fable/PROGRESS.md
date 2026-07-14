# Progress board — upstox-rust-sdk fork

_Last updated: 2026-07-14 (workspace created from NOB-HFT session 18
— see NOB F23: the absolute-path `[patch.crates-io]` finding led
here. Review not started.)_

## Done

- [x] docs/fable workspace + [DEEP-REVIEW-PLAN.md](DEEP-REVIEW-PLAN.md) created

## In progress

- (nothing claimed)

## Next up

1. [ ] Phase S0 — contracts & meta (constants vs current API docs; lib.rs surface vs NOB imports)
2. [ ] Phase S1 — money path (client auth/retry idempotency, rate_limiter table parity, order APIs)
3. [ ] Phase S2 — ws_client + proto decode
4. [ ] Phase S3 — remaining apis/models/utils
5. [ ] Phase S4 — hygiene (examples, one-shot scripts, .github/.vscode, README/CHANGELOG)
6. [ ] Phase S5 — NOB consumption contract table + suppression audit (also unblocks the NOB F23 vendoring-vs-git-dep decision)

## Ground rules

- Local gate + a NOB-HFT workspace build before every merge.
- Every manual Upstox-API sync gets a CHANGELOG entry + a dated
  parity note in the tracker.
