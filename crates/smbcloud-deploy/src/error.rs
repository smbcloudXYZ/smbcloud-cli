use thiserror::Error;

/// Errors surfaced by the deploy engine.
///
/// These are transport- and UI-neutral: a front-end decides how to render them
/// (a red spinner line in the CLI, a failed CI step, a git sideband error).
#[derive(Debug, Error)]
pub enum DeployError {
    /// No `.smb/config.toml` in the working directory. The front-end owns the
    /// interactive setup flow, so the engine reports this rather than prompting.
    #[error("no .smb/config.toml found; run setup first")]
    NeedsSetup,

    /// Couldn't determine the runtime from the working directory.
    #[error(
        "could not detect a runner: no package.json, Gemfile, Package.swift, or Cargo.toml found"
    )]
    RunnerNotDetected,

    /// Anything not yet modelled as a specific variant.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
