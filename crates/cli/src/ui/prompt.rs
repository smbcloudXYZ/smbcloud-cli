//! CI-aware interactive prompts.
//!
//! Thin wrappers over `dialoguer` that respect [`crate::ci::is_ci`]. In CI
//! (non-interactive) mode:
//!
//! * [`confirm`] resolves to its default instead of asking.
//! * [`input`] / [`input_optional`] return the supplied default, or fail.
//! * [`password`] and the required [`input_required`] fail fast — secrets and
//!   free-form answers can never be guessed.
//! * [`select`] / [`select_opt`] use an explicit CI default when the caller
//!   provides one, otherwise fail.
//!
//! Failures are plain [`anyhow::Error`]s with an actionable message, so a CI
//! pipeline gets a clear exit code instead of a hung TTY.

use {
    crate::{
        ci::{interactive_message, is_ci},
        interface::is_tui,
        ui::{confirm_dialog::confirm_delete_tui, fail_message},
    },
    anyhow::{anyhow, Result},
    console::style,
    dialoguer::{console::Term, theme::ColorfulTheme, Confirm, Input, Password, Select},
};

fn ci_required(what: &str) -> anyhow::Error {
    anyhow!(fail_message(&interactive_message(what)))
}

fn io_error(err: dialoguer::Error) -> anyhow::Error {
    anyhow!(fail_message(&format!("Prompt failed: {err}")))
}

/// Destructive-action confirmation, dispatched by interface. Refuses in CI
/// (deleting unconfirmed is never safe); renders the full-screen danger dialog
/// under `--tui`; otherwise asks inline with a `false` default. `what` names
/// the confirmation for the CI error; `message` is shown to the user.
pub fn confirm_delete(what: &str, message: &str) -> Result<bool> {
    if is_ci() {
        return Err(ci_required(what));
    }
    if is_tui() {
        return confirm_delete_tui(message).map_err(|e| anyhow!(fail_message(&e.to_string())));
    }
    confirm(message, false)
}

fn print_danger_warning(warning: &str) {
    println!();
    println!(
        "{}",
        style("⚠ This action cannot be undone.").red().bold()
    );
    println!("{warning}");
    println!();
}

/// Type-to-confirm deletion, mirroring the "type the resource name to
/// confirm" pattern used for irreversible deletes with real blast radius
/// (Vercel's project/team deletion, GitHub's repo deletion, …). A plain y/n
/// is too easy to reflex through; requiring the caller to type the resource's
/// own identifier back forces them to read it. Refuses in CI, like
/// [`confirm_delete`] — a typed confirmation can't be safely defaulted. Under
/// `--tui`, falls back to the boolean danger-zone dialog (no TUI text-input
/// widget yet).
pub fn confirm_delete_typed(what: &str, warning: &str, resource_identifier: &str) -> Result<bool> {
    if is_ci() {
        return Err(ci_required(what));
    }
    if is_tui() {
        return confirm_delete_tui(warning).map_err(|e| anyhow!(fail_message(&e.to_string())));
    }

    print_danger_warning(warning);

    let typed = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Type \"{resource_identifier}\" to confirm"))
        .allow_empty(true)
        .interact()
        .map_err(io_error)?;

    Ok(typed.trim() == resource_identifier)
}

/// Two-step type-to-confirm deletion for the highest-blast-radius actions —
/// ones that cascade across many owned resources (a tenant owns projects,
/// auth apps, mail apps, domains, …). First the caller types the resource's
/// own identifier (proves they're looking at the right thing), then a fixed
/// intent phrase (proves they mean to delete it, not just rename/inspect it).
/// Same CI/`--tui` handling as [`confirm_delete_typed`].
pub fn confirm_delete_double(
    what: &str,
    warning: &str,
    resource_identifier: &str,
    intent_phrase: &str,
) -> Result<bool> {
    if is_ci() {
        return Err(ci_required(what));
    }
    if is_tui() {
        return confirm_delete_tui(warning).map_err(|e| anyhow!(fail_message(&e.to_string())));
    }

    print_danger_warning(warning);

    let typed_identifier = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Type \"{resource_identifier}\" to confirm"))
        .allow_empty(true)
        .interact()
        .map_err(io_error)?;
    if typed_identifier.trim() != resource_identifier {
        return Ok(false);
    }

    let typed_phrase = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Type \"{intent_phrase}\" to confirm"))
        .allow_empty(true)
        .interact()
        .map_err(io_error)?;

    Ok(typed_phrase.trim() == intent_phrase)
}

/// Yes/no confirmation. In CI mode returns `default` without prompting.
pub fn confirm(prompt: &str, default: bool) -> Result<bool> {
    if is_ci() {
        return Ok(default);
    }
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(io_error)
}

/// Free-text input with a default. In CI mode returns the default.
pub fn input(prompt: &str, default: &str) -> Result<String> {
    if is_ci() {
        return Ok(default.to_string());
    }
    Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default.to_string())
        .interact()
        .map_err(io_error)
}

/// Free-text input that allows an empty value, with a default. In CI mode
/// returns the default (which may be empty).
pub fn input_optional(prompt: &str, default: &str) -> Result<String> {
    if is_ci() {
        return Ok(default.to_string());
    }
    Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default.to_string())
        .allow_empty(true)
        .interact()
        .map_err(io_error)
}

/// Required free-text input with no default. Fails in CI mode.
pub fn input_required(prompt: &str) -> Result<String> {
    if is_ci() {
        return Err(ci_required(prompt));
    }
    Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()
        .map_err(io_error)
}

/// Hidden password input. Always fails in CI mode — secrets are never guessed.
pub fn password(prompt: &str) -> Result<String> {
    if is_ci() {
        return Err(ci_required(prompt));
    }
    Password::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()
        .map_err(io_error)
}

/// Single-choice selection. In CI mode returns `ci_default` when the caller
/// supplies one, otherwise fails.
pub fn select<T: ToString>(
    prompt: &str,
    items: &[T],
    default: usize,
    ci_default: Option<usize>,
) -> Result<usize> {
    if is_ci() {
        return ci_default.ok_or_else(|| ci_required(prompt));
    }
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .default(default)
        .interact_on(&Term::stderr())
        .map_err(io_error)
}

/// Single-choice selection that may be cancelled (returns `None`). In CI mode
/// returns `Some(ci_default)` when supplied, otherwise fails.
pub fn select_opt<T: ToString>(
    prompt: &str,
    items: &[T],
    default: usize,
    ci_default: Option<usize>,
) -> Result<Option<usize>> {
    if is_ci() {
        return Ok(Some(ci_default.ok_or_else(|| ci_required(prompt))?));
    }
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .default(default)
        .interact_on_opt(&Term::stderr())
        .map_err(io_error)
}
