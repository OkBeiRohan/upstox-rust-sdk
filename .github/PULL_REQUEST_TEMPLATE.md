# Summary

<1-2 sentences explaining WHY this change exists, not WHAT (the diff is the WHAT)>

## Type

- [ ] feat
- [ ] fix
- [ ] chore
- [ ] docs
- [ ] refactor
- [ ] perf
- [ ] test
- [ ] build
- [ ] ci

## Scope

`<module>` (e.g. `client`, `rate-limiter`, `ws-client`, `orders`, `login`, `market-quote`, `models`, `constants`)

## Linked issues

- Closes: #<issue>
- Related: #<issue>

## Change checklist

- [ ] Conventional Commit message (`<type>(<scope>): <subject>` — subject explains WHY).
- [ ] No stubs: no `todo!()`, `unimplemented!()`, `panic!("not implemented")` in production code paths.
- [ ] No new `#[allow(...)]` or `#[expect(...)]` suppressions without a justifying comment + tracking issue.
- [ ] Doc comments (`///`) on every new public `fn`, `struct`, `enum`, `trait`.
- [ ] Tests in same commit as code — unit tests co-located in `#[cfg(test)] mod tests {}`.
- [ ] All pass locally:
  ```bash
  cargo fmt --check
  cargo clippy -- -D warnings
  cargo test
  cargo build
  ```
- [ ] No secrets committed (API keys, tokens, `.env` files with real values).
- [ ] CHANGELOG.md updated (for user-facing changes).

## Breaking changes

- [ ] No breaking changes.
- OR
- [ ] Breaking change: <describe what breaks and migration path>

## Test coverage

<describe what tests were added/updated, or why testing is not applicable>
