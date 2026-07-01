//! smbCloud build and deploy engine.
//!
//! This crate holds the deploy logic: runner detection, config resolution,
//! per-runtime build strategies, and transport. It has no front-end of its own.
//! The CLI, the CI action, and the server-side git receiver all drive the same
//! engine; they differ only in how they report progress and how they
//! authenticate.
//!
//! Two things are inverted so the engine stays reusable. [`Reporter`] takes the
//! place of direct `spinners`, `dialoguer`, and `println!` calls, so the engine
//! never owns the terminal. Auth is passed in (a token or credentials); the
//! engine never reads `~/.smb` or prompts for login.
//!
//! Interactive setup (creating or selecting a project, writing
//! `.smb/config.toml`) stays in the front-end. The engine takes a resolved
//! config, or returns [`DeployError::NeedsSetup`] for the caller to handle.

pub mod error;
pub mod known_hosts;
pub mod report;
pub mod runner;
pub mod transport;

pub use error::DeployError;
pub use report::{NoopReporter, Reporter};
pub use runner::detect_runner;
pub use transport::{RsyncTransport, Transport};
