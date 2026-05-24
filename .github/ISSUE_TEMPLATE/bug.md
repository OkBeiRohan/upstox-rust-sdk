---
name: Bug report
about: Report a defect in the SDK
title: 'bug: <short summary>'
labels: ['type/bug']
assignees: []
---

## Summary

<1-2 sentence description>

## Reproduction

1. <step>
2. <step>
3. <step>

### Minimal code example

```rust
// Paste a minimal reproducer here
```

## Expected

<what should happen>

## Actual

<what does happen — include error messages, panics, backtraces>

## Environment

- SDK version: `x.y.z`
- Rust version: `rustc --version`
- OS: <e.g. Ubuntu 24.04 / Windows 11 / macOS 15>
- Upstox account type: Standard / Plus
- Tokio runtime: single-threaded / multi-threaded

## Severity

- [ ] P0 — SDK panics, data loss, or incorrect order execution
- [ ] P1 — Feature broken but workaround exists
- [ ] P2 — Cosmetic, minor inconvenience

## Additional context

<any extra info — logs, screenshots, related issues>
