//! Rust client for the smbCloud GresIQ REST gateway.
//!
//! GresIQ is a managed-database layer inside smbCloud. It sits in front of
//! a PostgreSQL database, adds API-key auth, and exposes a simple REST
//! interface for inserting and querying rows.
//!
//! This crate handles the HTTP transport and auth headers. Schema knowledge
//! (which tables exist, what the rows look like) lives in the caller.
//!
//! # Quick start
//!
//! ```no_run
//! use smbcloud_gresiq_sdk::{Environment, GresiqClient, GresiqCredentials};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Hit { path: String, status: u16 }
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = GresiqClient::from_credentials(
//!         Environment::Dev,
//!         GresiqCredentials {
//!             api_key: "your-key",
//!             api_secret: "your-secret",
//!         },
//!     );
//!
//!     client.insert("hits", &Hit {
//!         path:   "/api/chat".into(),
//!         status: 200,
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```

mod client;
mod client_credentials;
mod error;
mod onde_apps;

pub use client::GresiqClient;
pub use client_credentials::{base_url, GresiqCredentials};
pub use error::GresiqError;
pub use onde_apps::{
    assign_model, create_app, list_apps, list_models, rename_app, OndeApp, OndeModel,
};
pub use smbcloud_network::environment::Environment;
