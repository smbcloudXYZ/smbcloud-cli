# GresIQ

GresIQ is smbCloud's managed PostgreSQL bootstrap crate for small and medium businesses. It is designed around the product shape teams expect from Neon, Prisma Postgres, and similar services:

- branch-aware environments
- autoscaling-friendly connection defaults
- safe connection-string handling
- a small Rust API that can sit underneath CLI, desktop, web, or future bindings

This crate is the Rust source of truth for bootstrapping a GresIQ connection and validating that a managed database endpoint is reachable.

## What the bootstrap includes

- `GresiqConfig` for loading a managed Postgres target from environment variables
- explicit smbCloud environment handling for `development`, `preview`, and `production`
- branch metadata for project, branch, and region aware workloads
- TLS-aware `tokio-postgres` connection setup
- redacted connection-string output for logs and CLI diagnostics
- a health-check query that returns database name, user, and server version

## Environment variables

`GresiqConfig::from_env()` reads:

```text
GRESIQ_PROJECT_SLUG
GRESIQ_HOST
GRESIQ_USERNAME
GRESIQ_PASSWORD
GRESIQ_BRANCH=main
GRESIQ_REGION=eu-north-1
GRESIQ_DATABASE=postgres
GRESIQ_PORT=5432
GRESIQ_SSL_MODE=require
GRESIQ_COMPUTE_TIER=autoscale
GRESIQ_APPLICATION_NAME=gresiq
GRESIQ_CONNECT_TIMEOUT_SECONDS=10
SMBCLOUD_ENVIRONMENT=development
```

## Example

```rust
use gresiq::GresiqConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = GresiqConfig::from_env()?;
    let connection = config.connect().await?;
    let health_check = connection.health_check().await?;

    println!("{}", config.redacted_connection_string());
    println!("{}", health_check.server_version);

    Ok(())
}
```

## CLI bootstrap

Run the crate binary directly to validate a managed endpoint:

```bash
cargo run -p gresiq
```

The binary prints:

- branch and region summary
- selected compute tier
- redacted endpoint URL
- database health-check details

## Product direction

The current bootstrap focuses on connection modeling and operational correctness. The next logical layer for GresIQ is:

- branch lifecycle APIs for preview environments
- pooled or proxy-based serverless connection routing
- passwordless or ephemeral credentials
- metrics, query insights, and tenant-aware usage controls
- adapters for smbCloud CLI, Tauri, wasm, npm, and native SDKs

More platform context is on the [smbCloud website](https://smbcloud.xyz/).

## License

Apache-2.0

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
