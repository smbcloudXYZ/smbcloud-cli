//! Global interface mode.
//!
//! `smb` presents through one of three interfaces, chosen once at startup and
//! stored here so the deep call tree can consult it without threading a value
//! through every function signature (the same approach as [`crate::ci`]):
//!
//! * [`Interface::Headless`] — the default. Line-based plain-text output; no
//!   full-screen takeover. Interactive prompts are still allowed on a TTY
//!   unless `--ci` is also set.
//! * [`Interface::Tui`] — full-screen `ratatui` views (`--tui`).
//! * [`Interface::Mcp`] — a Model Context Protocol server over stdio (`--mcp`).
//!   Implies non-interactive, structured output.
//!
//! `--tui` and `--mcp` are mutually exclusive.

use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interface {
    Headless,
    Tui,
    Mcp,
}

impl Interface {
    fn as_u8(self) -> u8 {
        match self {
            Interface::Headless => 0,
            Interface::Tui => 1,
            Interface::Mcp => 2,
        }
    }

    fn from_u8(value: u8) -> Interface {
        match value {
            1 => Interface::Tui,
            2 => Interface::Mcp,
            _ => Interface::Headless,
        }
    }
}

static INTERFACE: AtomicU8 = AtomicU8::new(0);

/// Set the active interface. Called once from `main`.
pub fn set_interface(interface: Interface) {
    INTERFACE.store(interface.as_u8(), Ordering::Relaxed);
}

/// The active interface.
pub fn current() -> Interface {
    Interface::from_u8(INTERFACE.load(Ordering::Relaxed))
}

/// Whether the full-screen TUI interface is active.
pub fn is_tui() -> bool {
    current() == Interface::Tui
}

/// Whether the MCP server interface is active.
pub fn is_mcp() -> bool {
    current() == Interface::Mcp
}

/// Whether the default headless interface is active.
pub fn is_headless() -> bool {
    current() == Interface::Headless
}

/// Resolve the interface from the parsed `--tui` / `--mcp` flags. The two flags
/// are mutually exclusive; passing both is an error.
pub fn resolve(tui: bool, mcp: bool) -> Result<Interface> {
    match (tui, mcp) {
        (true, true) => Err(anyhow!(
            "--tui and --mcp cannot be used together: choose one interface."
        )),
        (false, true) => Ok(Interface::Mcp),
        (true, false) => Ok(Interface::Tui),
        (false, false) => Ok(Interface::Headless),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_defaults_to_headless() {
        assert_eq!(resolve(false, false).unwrap(), Interface::Headless);
    }

    #[test]
    fn resolve_selects_tui_and_mcp() {
        assert_eq!(resolve(true, false).unwrap(), Interface::Tui);
        assert_eq!(resolve(false, true).unwrap(), Interface::Mcp);
    }

    #[test]
    fn resolve_rejects_both_flags() {
        assert!(resolve(true, true).is_err());
    }

    #[test]
    fn set_and_read_roundtrip() {
        set_interface(Interface::Tui);
        assert!(is_tui());
        assert!(!is_mcp());
        set_interface(Interface::Mcp);
        assert!(is_mcp());
        set_interface(Interface::Headless);
        assert!(is_headless());
    }
}
