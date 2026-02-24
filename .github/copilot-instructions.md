# GitHub Copilot Instructions — smbcloud-cli

## Project Overview

`smbcloud-cli` is a Rust CLI tool that ships as the `smb` binary. It lets users authenticate, manage projects, and trigger deployments on the smbCloud platform. The workspace targets Rust 1.93 (edition 2021) and is built as a Cargo workspace with multiple focused crates.

## Workspace Structure

| Crate | Role |
|---|---|
| `crates/cli` | Main binary (`smb`) — all commands, entrypoint |
| `crates/smbcloud-model` | Shared `serde` data models (source of truth for API shapes) |
| `crates/smbcloud-network` | Network config, environment resolution, connectivity check |
| `crates/smbcloud-networking` | Core HTTP client (`SmbClient`) |
| `crates/smbcloud-networking-account` | Account API calls — login, logout, me |
| `crates/smbcloud-networking-project` | Project API calls |
| `crates/smbcloud-utils` | Shared utility helpers |
| `crates/gresiq` | Internal tooling |

### CLI Source Layout (`crates/cli/src/`)

```
main.rs              — tokio entrypoint, logging setup, command dispatch
lib.rs               — crate root, SmbClient accessor
cli/                 — Cli struct, Commands enum, CommandResult
account/             — login, logout, me handlers
deploy/              — deploy command handler
project/             — project init, crud, process handlers
token/               — token storage/retrieval
ui/                  — terminal output helpers (spinners, styles)
```

## Dependency Graph

```
crates/cli
  └── smbcloud-networking-account
  └── smbcloud-networking-project
        └── smbcloud-networking  (SmbClient)
              └── smbcloud-model (API types)
  └── smbcloud-network  (Environment, connectivity)
  └── smbcloud-utils
```

All workspace-level dependencies are declared **once** in the root `Cargo.toml` under `[workspace.dependencies]`. Crate-level `Cargo.toml` files must inherit from workspace with `{ workspace = true }`.

## Coding Guidelines

### Correctness & Clarity

- Prioritize correctness and clarity. Performance is a secondary concern unless explicitly stated.
- Comments explain **why**, never **what**. Do not write comments that summarize or re-describe the code.
- Implement new functionality in existing files when it fits logically. Avoid creating many small, single-purpose files.

### Error Handling

- Use `?` to propagate errors. Never call `.unwrap()` or `.expect()` outside of test code.
- Never silently discard errors with `let _ =` on fallible operations:
  - Propagate with `?` when the caller should handle it
  - Use explicit `match` or `if let Err(e) = ...` when custom recovery logic is needed
  - Log and ignore only when truly intentional and visibility is preserved
- All command handlers return `anyhow::Result<CommandResult>`. Surface user-facing errors with `anyhow!("descriptive message")`.
- Ensure async errors propagate to the top level so the user sees meaningful terminal output.

### Naming & Style

- Use full words for all variable and parameter names — no single-letter names, no abbreviations (e.g. `environment` not `env`, `client` not `c`).
- Use variable shadowing to scope `.clone()` calls inside async closures, minimizing borrow lifetimes:
  ```rust
  cx.spawn({
      let handle = handle.clone();
      async move {
          handle.do_work().await;
      }
  });
  ```

### Module Structure

- **Never create `mod.rs` files.** Use `src/some_module.rs` instead of `src/some_module/mod.rs`.
- When creating new crates, set `[lib] path = "src/<crate_name>.rs"` in `Cargo.toml` instead of relying on the default `lib.rs`.

### Panics & Bounds

- Avoid any operation that can panic at runtime: no `.unwrap()`, no unchecked indexing, no `.expect()` in production code.
- When iterating or slicing, always validate bounds or use safe alternatives like `.get()`.

## Architecture Patterns

### Adding a New Command

1. Add a variant to the `Commands` enum in `crates/cli/src/cli/`
2. Create a handler function in `crates/cli/src/<domain>/`
3. Wire it in the `match cli.command { ... }` block in `main.rs`
4. Mark commands that require internet access in the `needs_internet` match block

### Adding a New API Call

1. Add or update the request/response types in `crates/smbcloud-model/src/`
2. Implement the call in the relevant `crates/smbcloud-networking-*/src/` crate using `SmbClient`
3. Call the networking function from the CLI command handler

### Environment & Auth

- The `Environment` type (from `smbcloud-network`) is threaded through all command handlers — always accept it as the first parameter.
- Auth tokens are managed through the `token` module — never hardcode credentials.
- The `CLI_CLIENT_SECRET` comes from the `.env` file via `dotenv_codegen` at compile time.

### Terminal Output

- Use the `ui` module helpers for consistent spinner, progress, and styled output.
- Use `console::style(...)` for coloured terminal text.
- Errors are printed in red with a `✘` prefix (handled in `main.rs`).

## Build & Test Commands

```bash
# Verify compilation across the whole workspace
cargo check --workspace

# Build release binary
cargo build --release

# Run all tests
cargo test --all-features

# Lint — treat warnings as errors
cargo clippy --workspace --tests -- -D warnings

# Format
cargo fmt --all

# Check formatting without modifying files
cargo fmt --all -- --check

# Publish all workspace crates (requires cargo-workspaces)
cargo workspaces publish --publish-as-is
```

## Common Pitfalls

1. **`.env` must exist** with `CLI_CLIENT_SECRET` before building. CI generates it from secrets; locally you must create it manually.
2. **No `mod.rs`** — always use named files for submodules.
3. **All new dependencies go in the root `Cargo.toml`** under `[workspace.dependencies]`, then inherit them in each crate with `{ workspace = true }`.
4. **`#[tokio::main]` lives only in `main.rs`** — all other async entry points are plain `async fn`.
5. **The binary is named `smb`**, not `smbcloud-cli` — it comes from `[[bin]] name = "smb"` in `crates/cli/Cargo.toml`.
6. **`SmbClient` is a zero-cost static reference** — obtain it with the `client()` helper in `lib.rs`, don't construct it ad-hoc.
7. **Do not add crate-local dependencies** without first checking the workspace root — duplicate version declarations break the workspace resolver.

## Pull Request Conventions

- Use a clear, imperative PR title: `Fix crash when project name is empty`, not `fixed stuff`.
- No conventional-commit prefixes (`feat:`, `fix:`, etc.) in the PR title.
- Include a `Release Notes:` section at the end of the PR body:
  ```
  Release Notes:

  - Fixed ...
  ```
  Use `- N/A` for non-user-facing changes.
