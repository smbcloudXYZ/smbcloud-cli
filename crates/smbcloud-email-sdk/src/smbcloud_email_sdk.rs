//! Rust client for the smbCloud transactional email API.
//!
//! Send transactional email from your verified domain and read each message's
//! delivery status. This crate handles the HTTP transport and the
//! `Authorization: Bearer <api_key>` header; you build the message and own the
//! content.
//!
//! # Quick start
//!
//! ```no_run
//! use smbcloud_email_sdk::{EmailClient, EmailCredentials, Environment, SendEmail};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = EmailClient::from_credentials(
//!         Environment::Production,
//!         EmailCredentials { api_key: "smb_mail_your_key" },
//!     );
//!
//!     let message = SendEmail::new("billing@example.com", ["customer@acme.com"])
//!         .subject("Your receipt")
//!         .html("<h1>Thanks!</h1>")
//!         .text("Thanks!")
//!         .idempotency_key("receipt-2026-0001");
//!
//!     let sent = client.send(&message).await?;
//!     println!("sent {} ({:?})", sent.id, sent.status);
//!
//!     Ok(())
//! }
//! ```

mod client;
mod client_credentials;
mod error;
mod message;

pub use client::EmailClient;
pub use client_credentials::{base_url, EmailCredentials};
pub use error::EmailError;
pub use message::{Attachment, EmailEvent, EmailMessage, EmailStatus, SendEmail};
pub use smbcloud_network::environment::Environment;
