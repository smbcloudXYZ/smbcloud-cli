# Contributing to smbcloud-cli

Thank you for considering contributing to smbcloud-cli! This document covers everything you need to get up and running.

## Code of Conduct

By participating in this project you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and considerate of others.

## Prerequisites

- [Rust](https://rustup.rs/) — toolchain version is pinned in `rust-toolchain.toml` (currently 1.93)
- [cargo-workspaces](https://github.com/pksunkara/cargo-workspaces) — for publishing (`cargo install cargo-workspaces`)
- A `.env` file in the repo root containing `CLI_CLIENT_SECRET=<value>` — required at compile time by `dotenv_codegen`

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork:
   ```sh
   git clone https://github.com/your-username/smbcloud-cli.git
   cd smbcloud-cli
   ```
3. Create the `.env` file (ask a maintainer for the value, or use a local stub for offline work):
   ```sh
   echo "CLI_CLIENT_SECRET=your_secret_here" > .env
   ```
4. Verify everything compiles:
   ```sh
   cargo check --workspace
   ```
5. Create a branch for your work:
   ```sh
   git checkout -b your-feature-or-fix
   ```

## Development Workflow

### Build

```sh
# Debug build
cargo build

# Release build
cargo build --release
```

The compiled binary is at `target/release/smb`.

### Test

```sh
cargo test --all-features
```

### Lint

```sh
cargo clippy --workspace --tests -- -D warnings
```

Fix any warnings before opening a PR — CI treats warnings as errors.

### Format

```sh
# Apply formatting
cargo fmt --all

# Check only (what CI runs)
cargo fmt --all -- --check
```

## Project Structure

This is a Cargo workspace. Each crate has a focused responsibility:

| Crate                                | Purpose                                    |
| ------------------------------------ | ------------------------------------------ |
| `crates/cli`                         | Main binary (`smb`) — commands, entrypoint |
| `crates/smbcloud-model`              | Shared API data types (`serde` structs)    |
| `crates/smbcloud-network`            | Network config and environment resolution  |
| `crates/smbcloud-networking`         | Core HTTP client (`SmbClient`)             |
| `crates/smbcloud-auth` | Auth SDK API calls                          |
| `crates/smbcloud-networking-project` | Project API calls                          |
| `crates/smbcloud-utils`              | Shared utilities                           |

All workspace-level dependencies are declared **once** in the root `Cargo.toml` under `[workspace.dependencies]`. When adding a dependency to a crate, add it to the root table first, then inherit it with `{ workspace = true }` in the crate's own `Cargo.toml`.

## Coding Guidelines

These rules are enforced in code review and by CI:

- Use `?` to propagate errors — never `.unwrap()` or `.expect()` in production code.
- Never silently discard errors with `let _ =` on fallible operations.
- Do not create `mod.rs` files — use `src/some_module.rs` instead of `src/some_module/mod.rs`.
- Use full words for variable names — no abbreviations.
- Comments explain _why_, not _what_. Do not write comments that summarize code.
- Prefer adding to existing files over creating many small new files.

The full guidelines are in [AGENTS.md](AGENTS.md).

## Submitting a Pull Request

1. Ensure all of the following pass locally before pushing:
   ```sh
   cargo check --workspace
   cargo test --all-features
   cargo clippy --workspace --tests -- -D warnings
   cargo fmt --all -- --check
   ```
2. Push your branch and open a PR against `main`.
3. Fill out the pull request template — in particular the **Release Notes** section.
4. A maintainer will review your changes and may request modifications.
5. Once approved, your PR will be merged.

### PR Title Convention

Use a clear, imperative title that describes the change:

- ✅ `Fix crash when project name contains spaces`
- ✅ `Add logout command`
- ❌ `feat: add logout` (no conventional-commit prefixes)
- ❌ `Fixed some stuff.` (no past tense, no trailing punctuation)

## Reporting Bugs

Use the GitHub issue tracker and fill out the bug report template. Please include:

- A clear description of the problem
- Steps to reproduce
- Expected vs. actual behaviour
- Your OS and the output of `smb --version`

## Requesting Features

Use the GitHub issue tracker and fill out the feature request template. Describe the use case clearly — what problem does the feature solve?

## Releasing

Releases are handled by maintainers using [cargo-workspaces](https://github.com/pksunkara/cargo-workspaces):

```sh
# Bump versions interactively
cargo workspaces version

# Publish all crates
cargo workspaces publish --publish-as-is
```

GitHub Actions workflows handle the GitHub release and npm wrapper publish automatically on tag push.

## Questions?

Open a GitHub issue or reach out to the maintainers. We're happy to help.
