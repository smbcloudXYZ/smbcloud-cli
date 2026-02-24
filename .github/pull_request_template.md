## Related Issue

Fixes # <!-- issue number, or remove this line if not applicable -->

## Description

<!-- What does this PR do? Why is this change needed? -->

## Type of Change

<!-- Mark the relevant option with an [x] -->

- [ ] Bug fix
- [ ] New feature
- [ ] Refactor (no functional change)
- [ ] Documentation update
- [ ] CI / tooling change

## How Has This Been Tested?

<!-- Describe the tests you ran or why tests are not applicable -->

- [ ] `cargo test --all-features`
- [ ] Manual testing (`smb <command>`)

## Checklist

- [ ] `cargo check --workspace` passes
- [ ] `cargo clippy --workspace --tests -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] No new `unwrap()` or `expect()` calls in production code
- [ ] No new `mod.rs` files introduced
- [ ] New dependencies added to root `Cargo.toml` and inherited with `{ workspace = true }`
- [ ] Error messages are user-friendly and surfaced to the terminal

## Release Notes

<!-- One bullet. Use "- Added ...", "- Fixed ...", or "- Improved ..." for user-facing changes. Use "- N/A" otherwise. -->

-
