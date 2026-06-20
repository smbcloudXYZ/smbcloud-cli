//! Global CI / non-interactive mode.
//!
//! When enabled (via the global `--ci` flag, `SMB_CI=1`, or the conventional
//! `CI` env var), interactive prompts are disabled: confirmations resolve to
//! their default, and any prompt that needs real user input fails fast with an
//! actionable error instead of blocking on a TTY that isn't there.
//!
//! The flag is parsed once in `main` and stored here so the deep call tree can
//! consult it without threading a boolean through every function signature.

use std::sync::atomic::{AtomicBool, Ordering};

static CI: AtomicBool = AtomicBool::new(false);

/// Enable or disable CI (non-interactive) mode. Called once from `main`.
pub fn set_ci(enabled: bool) {
    CI.store(enabled, Ordering::Relaxed);
}

/// Whether CI (non-interactive) mode is active.
pub fn is_ci() -> bool {
    CI.load(Ordering::Relaxed)
}

/// Standard message for a prompt that cannot run in CI mode. `what` names the
/// thing that needed input (e.g. "Project setup", "Login").
pub fn interactive_message(what: &str) -> String {
    format!(
        "{what} requires interactive input, but --ci (non-interactive) mode is on. \
         Provide it via a flag, an environment variable, or .smb/config.toml — \
         or run the command without --ci."
    )
}

/// Resolve CI mode from the parsed `--ci`/`SMB_CI` flag plus the conventional
/// `CI` environment variable that most CI providers set (`CI=true`/`1`).
pub fn resolve(flag: bool) -> bool {
    if flag {
        return true;
    }
    match std::env::var("CI") {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_forces_ci_without_touching_env() {
        // `--ci`/`SMB_CI` short-circuits before the conventional CI var is read.
        assert!(resolve(true));
    }

    #[test]
    fn set_and_read_roundtrip() {
        set_ci(true);
        assert!(is_ci());
        set_ci(false);
        assert!(!is_ci());
    }
}
