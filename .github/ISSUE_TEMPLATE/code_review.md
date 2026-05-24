---
name: Code review
about: Review a specific module or file group in the codebase
title: 'review: <module/file group>'
labels: ['type/code-review']
assignees: []
---

## Module / Files

| File | Lines | Has Tests? |
|------|-------|------------|
| `src/<path>` | N | Yes / No |

## Review checklist

### Code quality
- [ ] No `unwrap()` in production code paths without safety comment
- [ ] No `panic!()` / `todo!()` / `unimplemented!()` in production code
- [ ] Error handling uses `Result<T, E>` with meaningful error types
- [ ] No `#[allow(unused)]` or dead code
- [ ] No `unsafe` blocks without safety documentation

### Rust idioms
- [ ] Proper use of ownership, borrowing, lifetimes
- [ ] Builder pattern or `Default` where appropriate
- [ ] Enums with `#[non_exhaustive]` for public API stability
- [ ] `Clone` / `Copy` / `Debug` / `Display` derived where appropriate
- [ ] `pub` visibility is minimal — don't expose internals

### Documentation
- [ ] `///` doc comments on all public items
- [ ] Module-level `//!` documentation
- [ ] Examples in doc comments for complex APIs

### Testing
- [ ] Unit tests exist in `#[cfg(test)] mod tests {}`
- [ ] Edge cases covered (empty input, error paths, boundary values)
- [ ] Async tests use `#[tokio::test]`
- [ ] No tests that hit real network endpoints

## Unit test plan

<describe what unit tests should be added and for which functions>

## Findings

<to be filled during review>

## PR instructions

When submitting changes based on this review:
1. Branch: `review/<module-name>`
2. Commit: `refactor(<scope>): <what changed and why>`
3. Reference this issue: `Closes #<this-issue>`
4. Ensure `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes
