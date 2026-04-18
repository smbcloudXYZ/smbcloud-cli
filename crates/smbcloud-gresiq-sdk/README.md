# smbcloud-gresiq-sdk

Rust client for the **smbCloud GresIQ** REST gateway. API-key authenticated app management and model assignment for [Onde Inference](https://ondeinference.com).

## What is GresIQ?

GresIQ is a managed-database layer inside smbCloud. It sits in front of a PostgreSQL database, adds API-key auth, and exposes a simple REST interface. This SDK handles HTTP transport and auth headers.

## Installation

```toml
[dependencies]
smbcloud-gresiq-sdk = "0.3"
```

## Quick start

```rust
use smbcloud_gresiq_sdk::{Environment, GresiqClient, GresiqCredentials};
use serde::Serialize;

#[derive(Serialize)]
struct Hit { path: String, status: u16 }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = GresiqClient::from_credentials(
        Environment::Production,
        GresiqCredentials {
            api_key:    "your-api-key",
            api_secret: "your-api-secret",
        },
    );
    client.insert("hits", &Hit { path: "/api/chat".into(), status: 200 }).await?;
    Ok(())
}
```

## Onde Inference â app and model management

```rust
use smbcloud_gresiq_sdk::{
    Environment, list_apps, create_app, list_models, assign_model, rename_app,
};

// List all apps for the authenticated user
let apps = list_apps(&Environment::Production, APP_ID, APP_SECRET, &token).await?;

// Create a new app
let app = create_app(&Environment::Production, APP_ID, APP_SECRET, &token, "My App").await?;

// List available models and assign one
let models = list_models(&Environment::Production, APP_ID, APP_SECRET, &token).await?;
assign_model(&Environment::Production, APP_ID, APP_SECRET, &token, &app.id, &models[0].id).await?;

// Rename an existing app
rename_app(&Environment::Production, APP_ID, APP_SECRET, &token, &app.id, "Renamed App").await?;
```

## Key types

| Type                | Description                                                      |
| ------------------- | ---------------------------------------------------------------- |
| `GresiqClient`      | Low-level insert client for generic row writes                   |
| `GresiqCredentials` | API key and secret pair                                          |
| `GresiqError`       | `Http` (network failure) or `Api` (non-2xx with body)            |
| `OndeApp`           | Registered Onde app (id, name, status, secret, current model id) |
| `OndeModel`         | Catalog model (id, name, family, format, approx size bytes)      |
| `Environment`       | `Dev` or `Production`                                            |

## License

Apache-2.0
