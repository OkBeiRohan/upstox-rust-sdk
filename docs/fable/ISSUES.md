# upstox-rust-sdk issue registry (collaborative)

Same conventions as the
[NOB-HFT registry](https://gitlab.com/developers7143329/Nifty-Options-Buyer-HFT/-/blob/main/docs/fable/ISSUES.md):
one row per issue, append at the bottom of the relevant table, never
renumber. Statuses: `open` → `in-progress` → `fixed(<sha>)` |
`wontfix(reason)`.

Prefixes: **U** = SDK code issues · **UD** = doc rot · cross-link
NOB F/D/S items where a finding originates there.

## U — code

| ID | Title | Where | Status | Owner | Notes |
|----|-------|-------|--------|-------|-------|
| U1 | Full deep review pending — fork carries vendor-grade correctness responsibility (upstream unmaintained, manual API syncs) with no standing review trail | repo-wide | open | — | Execute [DEEP-REVIEW-PLAN.md](DEEP-REVIEW-PLAN.md) S0→S5. Filed from NOB F23 follow-up |

## UD — docs

| ID | Title | Where | Status | Owner | Notes |
|----|-------|-------|--------|-------|-------|
| UD1 | README/CHANGELOG posture unverified — must state fork purpose, divergence from upstream, and the NOB consumption contract | `README.md`, `CHANGELOG.md` | open | — | Verdict lands in phase S4 |
