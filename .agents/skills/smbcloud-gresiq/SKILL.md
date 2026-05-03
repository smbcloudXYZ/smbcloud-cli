---
name: smbcloud-gresiq
description: Use when building, debugging, or extending GresIQ, smbCloud's managed PostgreSQL service, especially the Rust crate in `crates/smbcloud-gresiq`, managed connection bootstrap, branch-aware database config, TLS-aware `tokio-postgres` setup, environment-variable loading, and future Neon or Prisma-style service capabilities such as preview branches, pooled connections, and ephemeral credentials.
---

# smbCloud GresIQ

Use this skill when the task is about GresIQ, smbCloud's managed PostgreSQL service.

Applies to:

- `crates/smbcloud-gresiq/Cargo.toml`
- `crates/smbcloud-gresiq/src/gresiq.rs`
- `crates/smbcloud-gresiq/src/main.rs`
- `crates/smbcloud-gresiq/README.md`
- workspace wiring for new GresIQ dependencies
- managed Postgres bootstrap design, connection policy, and product-shape changes

## Scope boundary

This skill is for the managed Postgres service layer and its Rust bootstrap crate.

Do not use it for:

- smbCloud auth flows unless the database service change directly affects auth integration
- generic deploy logic
- unrelated CLI packaging or release automation

Use the relevant smbCloud auth, deploy, or release skills when those are the actual center of gravity.

## Source of truth

Start from these files:

- `crates/smbcloud-gresiq/src/gresiq.rs`
- `crates/smbcloud-gresiq/src/main.rs`
- `crates/smbcloud-gresiq/Cargo.toml`
- `crates/smbcloud-gresiq/README.md`
- workspace `Cargo.toml` when dependency or member wiring changes

Treat the Rust crate as the source of truth. Keep future CLI, wasm, Tauri, npm, or native wrappers thin.

## Current service model

The current GresIQ bootstrap models:

- smbCloud environments: `development`, `preview`, `production`
- branch-aware database targets
- endpoint host, port, and SSL mode
- credentials and database name
- connection policy such as compute tier, application name, and connect timeout
- redacted connection-string output
- a connectivity health check using `tokio-postgres`

Preserve these properties unless the task explicitly changes the product model.

## Product direction

Take inspiration from Neon, Prisma Postgres, and similar managed database products, but fit the smbCloud platform rather than copying their APIs blindly.

Good directions:

- preview branches per app environment
- autoscaling or burst compute defaults
- pooled or proxy-backed connection paths for serverless runtimes
- safe credential handling and redacted logs
- ephemeral or tenant-scoped credentials
- operational checks that surface actionable failures

Avoid adding speculative product features with no clear interface or storage model.

## Implementation guidance

### Crate shape

Prefer a library-first design:

- reusable Rust API in `src/gresiq.rs`
- thin binary in `src/main.rs`

If additional surfaces are added later, bind to the existing Rust API rather than duplicating logic.

### Configuration

Make environment loading explicit and typed.

Expected config areas:

- smbCloud environment
- project slug
- branch name
- region
- host and port
- SSL mode
- username, password, and database name
- compute or connection policy

Do not hardcode localhost defaults except in intentionally local examples.

### Connections

Prefer correctness over cleverness.

- use TLS when the endpoint requires it
- keep connection-string generation available for integration points
- provide a redacted form for logs and CLI output
- propagate errors instead of hiding them behind generic failures

If adding pooling, document whether the crate owns pooling or whether it only models direct connections.

### Errors

Use typed error enums for:

- invalid configuration
- invalid environment variable values
- TLS setup failures
- PostgreSQL connection or query failures

Do not use `unwrap()` in the service path.

## Validation

Use the smallest relevant checks first.

### Rust

- `cargo fmt --package gresiq`
- `cargo check -p gresiq`
- `cargo test -p gresiq`

### Runtime

If the task changes live connection behavior, validate with a real managed endpoint when credentials are available:

- set `GRESIQ_*` environment variables
- run `cargo run -p gresiq`

Do not claim runtime validation happened unless a real endpoint was exercised.

## Common mistakes

- treating GresIQ like a one-off example binary instead of a reusable service crate
- logging raw passwords or full unredacted connection strings
- hardcoding insecure defaults for production-facing paths
- mixing up smbCloud environment semantics with database branch semantics
- adding platform wrappers that duplicate the Rust logic
- introducing pooling, proxy, or branch-management claims in the README without implementing the underlying API
