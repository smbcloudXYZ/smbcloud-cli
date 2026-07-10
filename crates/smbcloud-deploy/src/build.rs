//! Local build strategies.
//!
//! A [`BuildStrategy`] runs a project's build locally and reports where the
//! shippable output is, as a [`BuildArtifact`]. Transport (see
//! [`crate::transport`]) takes it from there. Keeping "build" and "ship"
//! separate is what lets the same engine drive a local build, a CI build, or a
//! server-side build without the strategies knowing which.

use crate::{error::DeployError, report::Reporter};
use anyhow::anyhow;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// What a build produced and where to ship it from.
pub struct BuildArtifact {
    /// Local directory whose contents should be shipped.
    pub source_dir: PathBuf,
}

/// Builds a project locally into a [`BuildArtifact`].
pub trait BuildStrategy {
    fn build(&self, reporter: &dyn Reporter) -> Result<BuildArtifact, DeployError>;
}

/// A Vite / SPA build: run `<package_manager> build` in the project directory
/// and ship its output directory (e.g. `dist`).
pub struct ViteSpaBuild {
    pub project_path: String,
    pub output_dir: String,
    pub package_manager: String,
}

impl BuildStrategy for ViteSpaBuild {
    fn build(&self, reporter: &dyn Reporter) -> Result<BuildArtifact, DeployError> {
        reporter.step_start(&format!(
            "Building {} with {}…",
            self.project_path, self.package_manager
        ));

        // Validate up front: `current_dir` on a missing path only surfaces as a
        // confusing spawn error at status() time.
        let project_dir = Path::new(&self.project_path);
        if !project_dir.exists() {
            reporter.step_fail(&format!(
                "Project path '{}' does not exist.",
                self.project_path
            ));
            return Err(DeployError::Other(anyhow!(
                "Project path '{}' does not exist. Check the 'source' field in .smb/config.toml.",
                self.project_path
            )));
        }

        let status = Command::new(&self.package_manager)
            .arg("build")
            .current_dir(&self.project_path)
            .status()
            .map_err(|e| {
                reporter.step_fail(&format!("Failed to spawn '{}': {e}", self.package_manager));
                DeployError::Other(anyhow!("Failed to spawn '{}': {e}", self.package_manager))
            })?;

        if !status.success() {
            reporter.step_fail("Build failed. See output above.");
            return Err(DeployError::Other(anyhow!(
                "'{} build' exited with status {status}",
                self.package_manager
            )));
        }

        reporter.step_done("Build complete.");
        Ok(BuildArtifact {
            source_dir: project_dir.join(&self.output_dir),
        })
    }
}
