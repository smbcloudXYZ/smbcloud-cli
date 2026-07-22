# Public, open-source repository

`smbcloud-cli` is published on GitHub as **open source**. Everything you commit here — code, comments, docs, and `.agents/skills/*` — is **world-readable and permanent** (git history outlives any later deletion). Before writing anything, ask: "is this safe on a public repo forever?"

Do **not** add internal smbCloud infrastructure detail or secrets:

- server hostnames/IPs and operational endpoints beyond what the CLI already targets in source — e.g. account-scoped SSH key names (`id_<n>@smbcloud`), `api-1.smbcloud.xyz`, internal health ports
- account/user IDs and which user or tenant owns which project
- real customer/app domains, PM2 process names, the production port → app → domain registry
- workspace/project IDs and `frontend_app_id` / `deploy_repo_id` values
- secrets/config: API keys, tokens, `.env` values, connection strings, real auth/CORS origins
- commands that read local credentials to enumerate the API (e.g. `cat ~/.smb/token | curl …`)
- incident logs or examples that name real apps, tenants, customers, or dated rollouts

Keep docs and skills **generic** — describe how the tool behaves, using placeholders (`example.com`, `<app>`, `<source>`, `<port>`, `<n>`). The base API host (`api.smbcloud.xyz`) is already in the source, so it is not a new leak; the items above are. Fleet-specific operational detail and the internal deploy reference live only in the private `smbcloud` repo.

---

# Rust coding guidelines

- Prioritize code correctness and clarity. Speed and efficiency are secondary priorities unless otherwise specified.
- Do not write organizational or comments that summarize the code. Comments should only be written in order to explain "why" the code is written in some way in the case there is a reason that is tricky / non-obvious.
- Prefer implementing functionality in existing files unless it is a new logical component. Avoid creating many small files.
- Avoid using functions that panic like `unwrap()`, instead use mechanisms like `?` to propagate errors.
- Be careful with operations like indexing which may panic if the indexes are out of bounds.
- Never silently discard errors with `let _ =` on fallible operations. Always handle errors appropriately:
  - Propagate errors with `?` when the calling function should handle them
  - Use `.log_err()` or similar when you need to ignore errors but want visibility
  - Use explicit error handling with `match` or `if let Err(...)` when you need custom logic
  - Example: avoid `let _ = client.request(...).await?;` - use `client.request(...).await?;` instead
- When implementing async operations that may fail, ensure errors propagate to the UI layer so users get meaningful feedback.
- Never create files with `mod.rs` paths - prefer `src/some_module.rs` instead of `src/some_module/mod.rs`.
- When creating new crates, prefer specifying the library root path in `Cargo.toml` using `[lib] path = "...rs"` instead of the default `lib.rs`, to maintain consistent and descriptive naming (e.g., `gpui.rs` or `main.rs`).
- Avoid creative additions unless explicitly requested
- Use full words for variable names (no abbreviations like "q" for "queue")
- Use variable shadowing to scope clones in async contexts for clarity, minimizing the lifetime of borrowed references.
  Example:
  ```rust
  executor.spawn({
      let task_ran = task_ran.clone();
      async move {
          *task_ran.borrow_mut() = true;
      }
  });
  ```

---

# Build, test, and run

This is a Cargo workspace pinned to stable Rust (`rust-toolchain.toml`). The CLI binary is named **`smb`** (crate `smbcloud-cli`, in `crates/cli`).

```sh
cargo build                              # build everything (debug)
cargo run -p smbcloud-cli -- <args>      # run the CLI, e.g. -- me, -- deploy, -- --help
cargo run -p smbcloud-cli -- -e dev me   # talk to a local API (Environment::Dev -> http://localhost:8088)
```

CI gate (mirror it locally before pushing — see `.github/workflows/ci.yml`):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --exclude smbcloud-auth-sdk-wasm --tests -- -D warnings   # warnings are denied
cargo test  --workspace --exclude smbcloud-auth-sdk-wasm
cargo test -p smbcloud-model app_auth::tests::some_test    # a single test
```

**Always run `cargo fmt --all` and the `cargo clippy` command above before considering any
Rust change done** — after every edit, not just before a push. Fix clippy findings rather
than silencing them with `#[allow(...)]` unless there's a specific reason the lint doesn't
apply, and note that reason inline.

`ci.yml` runs six jobs on every push; run the ones relevant to what changed, not just the
main Rust workspace job — a change that only compiles in the main workspace but breaks one
of these is still a broken CI run:

| Job | Touches | Local check |
|---|---|---|
| Rust workspace | any `crates/*` change (except the wasm crate) | `cargo fmt --all -- --check && cargo clippy --workspace --exclude smbcloud-auth-sdk-wasm --tests -- -D warnings && cargo test --workspace --exclude smbcloud-auth-sdk-wasm` |
| `smbcloud-auth-sdk-wasm` | that crate, or `smbcloud-auth-sdk` it wraps | `cargo check --package smbcloud-auth-sdk-wasm --target wasm32-unknown-unknown` |
| Python SDK | `sdk/python`, or `smbcloud-auth-sdk` | `cd sdk/python && maturin build --release --locked --out dist` |
| npm/WASM SDK | `sdk/npm/smbcloud-auth`, or `smbcloud-auth-sdk` | `cd sdk/npm/smbcloud-auth && node ./prepare-package.mjs` (needs `wasm-pack` installed) |
| NuGet .NET tool | `nuget/smbcloud-cli` | `dotnet build nuget/smbcloud-cli/SmbCloud.Cli.csproj --configuration Release` |
| Ruby gem | `sdk/gems/auth`, or `smbcloud-auth-sdk` | `cd sdk/gems/auth && bundle exec rake compile` |

The last four jobs need their own toolchain (maturin/PyO3, wasm-pack + Node, the .NET SDK,
Ruby/bundler) — if it isn't installed, say so explicitly rather than skipping the check
silently, so the user knows it wasn't verified.

`smbcloud-auth-sdk-wasm` targets `wasm32-unknown-unknown` and cannot build or test on the host — it is always excluded from workspace clippy/test and checked separately: `cargo check -p smbcloud-auth-sdk-wasm --target wasm32-unknown-unknown`.

`CLI_CLIENT_SECRET` (see `.env.example`) is the OAuth client-credentials secret read at runtime; tests and `cargo check` don't need it, but login flows against a real API do.

## Releases

Releases are prepared on the `development` branch with a clean tree via the Makefile, which drives `cargo workspaces` to bump every crate in lockstep, then syncs the package-manager manifests (npm/nuget/pypi) and regenerates the SDK lockfiles:

```sh
make patch | make minor | make major | make custom VERSION=0.x.y
```

It commits `Release <version>` and tags `v<version>` locally; pushing is manual. Crate publishing is separate and manual (`cargo workspaces publish`, see `docs/development.md`). The per-target release workflows (`release-*.yml`) build the distributable artifacts.

`server.json` (the MCP Registry listing) is version-synced by the same Makefile step, and `release-nuget.yml` triggers `release-mcp-registry.yml` once its publish job succeeds — the listing needs both npm and NuGet live at that version, and the registry validates its metadata against the real packages. See `docs/mcp-registry.md`.

---

# Architecture

`smb` is a thin async (`tokio`) `clap` front-end over a set of `smbcloud-*` library crates that talk to the smbCloud REST API. The product purpose is **deploying apps** (Node.js/Next.js, Ruby/Rails, Rust, Swift/Vapor, static/Vite) from the terminal.

## Request flow

`crates/cli/src/main.rs` → `cli/mod.rs` (the `Cli`/`Commands` clap tree) → a `process_*` handler under `account/`, `project/`, `deploy/`, or `mail/`. Each handler returns a `CommandResult` (a spinner + final symbol/message); `main` prints it and sets the exit code. The top-level `--environment` (`dev`/`production`, default production) threads through every handler and selects the API host and the on-disk state dir (`.smb` vs `.smb-dev`, see `smbcloud-network/src/environment.rs`).

Two cross-cutting globals are resolved once in `main` before any handler runs:
- **CI mode** (`crates/cli/src/ci.rs`): `--ci` / `SMB_CI=1` / conventional `CI` env. Stored in a process-global `AtomicBool` (`ci::is_ci()`) rather than threaded through signatures, so deep prompt code can fail fast instead of blocking on a missing TTY. New interactive prompts must check `is_ci()` and use `ci::interactive_message(...)`.
- **Logging**: bunyan JSON to `~/<smb_dir>/smbcloud-cli.log` (not stdout); user-facing output is `console`/spinner styling.

## Crate layout (workspace)

- `smbcloud-model` — shared serde types (`Project`, `User`, `Runner`, deploy config, `ErrorCode`/`ErrorResponse`). `crate-type = ["cdylib","rlib"]` because the SDK crates re-export it. Most unit tests live here.
- `smbcloud-network` — `Environment` enum (host/protocol/state-dir resolution) and connectivity checks.
- `smbcloud-networking` — `SmbClient` (the OAuth client identity) and the low-level HTTP client/constants.
- `smbcloud-networking-project` — typed REST calls for projects, deploy repos, frontend apps, deploy config, and deployments (the `crud_*` files).
- `smbcloud-auth` / `smbcloud-auth-sdk` — auth flows (login, signup, OIDC, Apple, client-credentials, reset/verify). `auth` is the in-CLI implementation; `auth-sdk` is the embeddable client.
- `smbcloud-auth-sdk-wasm` / `-py` (+ `sdk/`) — language bindings: WASM/`wasm-bindgen` for npm, PyO3 `cdylib` for Python, and a separate `rb_sys` Cargo workspace under `sdk/gems` for the Ruby gem. These wrap `smbcloud-auth-sdk`.
- `smbcloud-mail`, `smbcloud-gresiq`(`-sdk`), `smbcloud-s6n` — clients for the Mail, GresIQ (serverless Postgres), and S6n (S3-compatible storage) products.
- `smbcloud-utils` — `.smb/config.toml` (`Config`) parsing and SSH key-path resolution.

## Deploy subsystem (`crates/cli/src/deploy/`)

The most involved area. `process_deploy.rs` loads `.smb/config.toml` (running interactive `setup_project` if missing), overlays server-side config, validates project access, then **dispatches by `config.project.kind`** to a per-stack path: `vite-spa`, `nextjs-ssr`, `rails`, `rust`, `swift` each have their own `process_deploy_*.rs`. If `kind` is unset it falls back to `deployment_method`: `Rsync` (build locally, upload over rsync, restart over SSH) or `Git` (push to a remote git hook), with `detect_runner.rs` sniffing the framework from `package.json`/`Gemfile`/`Package.swift`/`Cargo.toml`.

**Monorepo**: `runner = Monorepo` with a `[[projects]]` array in config; `smb deploy --project <name>` (or an interactive picker) swaps `config.project` to the named sub-project before the dispatch above. `migrate.rs`/`smb migrate` pushes local deploy fields up to the server.

When editing a deploy path, keep the public-repo policy above in mind: SSH key names, hosts, ports, app/tenant identifiers, and PM2 process names must stay as placeholders — concrete fleet detail belongs only in the private `smbcloud` repo.
