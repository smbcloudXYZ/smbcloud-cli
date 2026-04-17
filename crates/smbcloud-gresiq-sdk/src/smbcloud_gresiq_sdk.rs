//! Rust client for the smbCloud GresIQ REST gateway.
//!
//! GresIQ is a managed-database layer inside smbCloud. It sits in front of
//! a PostgreSQL database, adds API-key auth, and exposes a simple REST
//! interface for inserting and querying rows.
//!
//! This crate handles the HTTP transport and auth headers. Schema knowledge
//! (which tables exist, what the rows look like) lives in the caller — see
//! `onde::pulse` for an example of wrapping this for a specific schema.
//!
//! # Quick start
//!
//! ```no_run
//! use smbcloud_gresiq_sdk::GresiqClient;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Hit { path: String, status: u16 }
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = GresiqClient::from_env()
//!         .expect("set GRESIQ_BASE_URL, GRESIQ_API_KEY, GRESIQ_API_SECRET");
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
mod error;

pub use client::GresiqClient;
pub use error::GresiqError;
