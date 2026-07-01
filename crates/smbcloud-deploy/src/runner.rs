//! Runner (runtime) detection for a working directory.

use crate::{error::DeployError, report::Reporter};
use smbcloud_model::runner::Runner;
use smbcloud_utils::config::Config;
use std::{env::current_dir, path::Path};

/// Detect which runtime this project uses: Next.js, Rails, Rust, Swift, or a
/// build-less static site.
///
/// The detection itself lives in [`Runner::from`]; this wraps it with progress
/// reporting and the monorepo short-circuit. It owns no spinner and prints
/// nothing directly. All progress goes through `reporter`, so the same call
/// works under the CLI, in CI, or on the server.
pub fn detect_runner(config: &Config, reporter: &dyn Reporter) -> Result<Runner, DeployError> {
    reporter.step_start("Checking runner");

    // A monorepo declares its runner explicitly and defers per-app detection to
    // the selected `[[projects]]` entry.
    if config.project.runner == Runner::Monorepo && config.projects.is_some() {
        reporter.step_done("Monorepo universal runner");
        return Ok(Runner::Monorepo);
    }

    let path = current_dir().map_err(|_| {
        reporter.step_fail("Could not read the current directory.");
        DeployError::RunnerNotDetected
    })?;

    let runner = Runner::from(&path).map_err(|_| {
        reporter.step_fail("Could not detect a runner.");
        DeployError::RunnerNotDetected
    })?;

    reporter.step_done(label_for(runner));
    Ok(runner)
}

/// Human-readable confirmation line for a detected runner.
fn label_for(runner: Runner) -> &'static str {
    match runner {
        Runner::Monorepo => "Monorepo universal runner",
        Runner::NodeJs => "Node.js runner detected",
        Runner::Static => "Static site, no build step required",
        Runner::Ruby if Path::new("Gemfile").exists() => "Ruby runner with Rails app detected",
        Runner::Swift if Path::new("Package.swift").exists() => {
            "Swift runner with Vapor app detected"
        }
        Runner::Rust if Path::new("Cargo.toml").exists() => "Rust runner with Cargo project detected",
        _ => "Runner detected",
    }
}
