# Session log — upstox-rust-sdk fork

Append-only. One `##` block per working session. Newest at the top.

---

## 2026-07-14 — Claude (Fable via GitLab Duo Chat) — session 1: workspace bootstrap

**Origin:** NOB-HFT deep-review finding F23 (absolute local path in
`[patch.crates-io]`) surfaced that this fork is the single most
safety-critical dependency with the least review coverage. Operator
confirmed: upstream is unmaintained, the fork is hand-synced against
the live Upstox API, and asked for a standing review workspace +
full review plan here.

**Created:** README (working agreement — note the merge gate
includes building the NOB workspace against the changed fork),
DEEP-REVIEW-PLAN (phases S0→S5, NOB's 11 dimensions + new dimension
12 "upstream/API parity" requiring a doc-URL+date verdict per
endpoint/constant), ISSUES (U/UD registry seeded with U1/UD1),
PROGRESS board.

**Structure observed (for the next session):** src/{client,
ws_client, rate_limiter, constants, lib}.rs + apis/ + models/ +
protos/ + utils/; root carries one-shot repo-automation scripts
(create_issues.{ps1,py}, map_*.py) and .vscode/ — phase S4 hygiene
candidates. `.github/` may hold upstream CI leftovers.

**Not done (deliberately):** no code read yet — the plan lands
first so review claims stay traceable, same discipline as NOB.

---
