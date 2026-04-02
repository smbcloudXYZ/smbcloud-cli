---
name: rust-cross-platform
description: Use when building or debugging Rust code that targets multiple platforms or language surfaces, especially Tauri, wasm/npm, UniFFI, Swift/Kotlin bindings, shared SDK crates, deployment-target issues, and environment parity across desktop, mobile, CLI, and web.
---

# Rust Cross Platform

Use this skill when a change spans Rust plus any of:

- Tauri desktop or mobile
- wasm or npm packaging
- UniFFI-generated Swift/Kotlin bindings
- shared SDK crates used by web, native, and CLI clients
- Apple platform linker or deployment-target issues
- environment-specific credentials or runtime configuration

## Default approach

Treat the Rust crate as the source of truth and keep platform adapters thin.

Preferred layering:

1. Shared Rust domain crate
2. Platform wrapper crate or binding layer
3. App-specific UI or shell code

Do not duplicate API contracts in TypeScript, Swift, Kotlin, and Rust if the Rust crate can own the contract.

## Environment parity

Always make environment selection explicit.

Use:

- an enum in Rust for `development` vs `production`
- one place for app ids, hosts, and secrets
- thin adapters in Tauri, wasm, or UniFFI to pass the enum through

Do not let frontend-only checks become the source of truth for runtime mode if Rust owns the actual behavior.

## Tauri conventions

For auth or networking features:

- keep logic in `src-tauri/src/...`
- expose narrow Tauri commands
- store session state in plugin-store, not only React state
- use Rust for build-mode checks such as `cfg!(debug_assertions)`

### macOS

`tauri.conf.json` `minimumSystemVersion` does not fix `tauri dev`.

If Swift-linked plugins are present, add `src-tauri/.cargo/config.toml` with:

- `MACOSX_DEPLOYMENT_TARGET`
- `-mmacosx-version-min=...` for `aarch64-apple-darwin`
- `-mmacosx-version-min=...` for `x86_64-apple-darwin`
- host `CC` and `CXX` if native deps might pick the wrong toolchain

If runtime fails on `libswift_Concurrency.dylib`, fix Cargo/macOS config before touching app code.

## wasm and npm conventions

For browser-facing Rust SDKs:

- keep the core logic in a normal Rust crate
- expose wasm bindings in a dedicated wrapper crate
- make wasm functions explicit and app-oriented
- prefer passing primitive strings and enums over complex borrowed structs

Typical exported operations:

- `signup_with_client`
- `login_with_client`
- `me_with_client`
- `remove_with_client`

### Packaging

Use `wasm-pack` for npm distribution unless there is a strong reason not to.

Recommended pattern:

- Rust crate under `crates/...-wasm`
- `wasm-bindgen` exports only the browser surface
- README contains exact browser import example

Do not make the HTML or JS app reimplement request signing or endpoint construction if the Rust crate already owns it.

## UniFFI conventions

Use UniFFI when Swift/Kotlin consumers need a stable SDK surface from Rust.

Guidelines:

- keep FFI-safe types small and explicit
- avoid leaking internal crate structure into the public interface
- prefer stringly transport only at the boundary, not internally
- map platform-facing errors into a deliberate error enum
- keep async boundaries clear; document whether the caller must dispatch off the main thread

When designing UniFFI APIs:

- expose nouns and use cases, not transport details
- keep constructors simple
- prefer one shared config object if several methods need the same environment or credentials

## Shared auth and SDK work

For auth-backed SDKs:

- keep account flows in the shared crate
- keep app credentials and environment resolution in one place
- make desktop, mobile, and web call the same Rust path where possible

If a web client must use wasm, bind the existing Rust account crate instead of rebuilding the flow in JavaScript.

## Validation

Use the smallest relevant checks.

### Rust

- `cargo check -p <crate>`
- `cargo test -p <crate>` when tests exist and are relevant

### Tauri

- `cargo check` in `src-tauri`
- frontend build, usually `pnpm build`

### wasm

- `cargo check -p <wasm-crate>`
- `wasm-pack build --target web` when packaging behavior changed

### UniFFI

- regenerate bindings if applicable
- build at least one native target that consumes the bindings

## Common mistakes

- duplicating business logic in each platform adapter
- hiding environment selection in UI-only code
- using app secrets in public web clients without noting the security tradeoff
- assuming `tauri.conf.json` fixes macOS dev runtime issues
- exposing unstable internal Rust types directly through wasm or UniFFI
- changing backend error shapes without updating parser code in wrappers
