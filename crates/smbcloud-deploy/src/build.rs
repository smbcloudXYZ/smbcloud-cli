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

/// Which SvelteKit adapter a project declares in its `package.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvelteKitAdapter {
    /// `@sveltejs/adapter-static`: prerendered pages plus an optional SPA
    /// fallback, served as plain files by nginx.
    Static,
    /// Any other `@sveltejs/adapter-*` package, e.g. `adapter-node` or
    /// `adapter-auto`.
    Other(String),
    /// `@sveltejs/kit` is present but no adapter package is.
    Missing,
}

impl SvelteKitAdapter {
    /// Reads the adapter from `package.json` contents. Returns `None` when the
    /// manifest doesn't depend on `@sveltejs/kit` at all.
    pub fn from_package_json(contents: &str) -> Result<Option<Self>, serde_json::Error> {
        let manifest: serde_json::Value = serde_json::from_str(contents)?;
        let dependency_names: Vec<&str> = ["dependencies", "devDependencies"]
            .iter()
            .filter_map(|section| manifest.get(section).and_then(|value| value.as_object()))
            .flat_map(|section| section.keys().map(String::as_str))
            .collect();

        if !dependency_names.contains(&"@sveltejs/kit") {
            return Ok(None);
        }

        let adapters: Vec<&str> = dependency_names
            .iter()
            .copied()
            .filter(|name| name.starts_with("@sveltejs/adapter-"))
            .collect();

        // A project can list several adapters (e.g. one per target); static
        // wins because it's the only one this strategy can ship.
        if adapters.contains(&"@sveltejs/adapter-static") {
            return Ok(Some(SvelteKitAdapter::Static));
        }
        Ok(Some(match adapters.first() {
            Some(name) => SvelteKitAdapter::Other((*name).to_string()),
            None => SvelteKitAdapter::Missing,
        }))
    }
}

/// A SvelteKit build using `@sveltejs/adapter-static`: install dependencies,
/// run `<package_manager> run build`, and ship the adapter's output directory
/// (`build` by default).
///
/// Server-rendered adapters are rejected up front. Their output needs a Node
/// process on the server, which this static path doesn't manage.
pub struct SvelteKitBuild {
    pub project_path: String,
    pub output_dir: String,
    pub package_manager: String,
}

impl SvelteKitBuild {
    fn fail(&self, reporter: &dyn Reporter, message: String) -> DeployError {
        reporter.step_fail(&message);
        DeployError::Other(anyhow!(message))
    }

    fn run(&self, reporter: &dyn Reporter, arguments: &[&str]) -> Result<(), DeployError> {
        let display = format!("{} {}", self.package_manager, arguments.join(" "));
        // Inherited stdio keeps the child on the user's terminal, so package
        // manager prompts (e.g. pnpm's modules purge) can still be answered.
        let status = Command::new(&self.package_manager)
            .args(arguments)
            .current_dir(&self.project_path)
            .status()
            .map_err(|error| {
                self.fail(
                    reporter,
                    format!("Failed to spawn '{}': {error}", self.package_manager),
                )
            })?;
        if !status.success() {
            return Err(self.fail(reporter, format!("'{display}' exited with status {status}")));
        }
        Ok(())
    }
}

impl BuildStrategy for SvelteKitBuild {
    fn build(&self, reporter: &dyn Reporter) -> Result<BuildArtifact, DeployError> {
        reporter.step_start(&format!(
            "Building SvelteKit app {} with {}…",
            self.project_path, self.package_manager
        ));

        let project_dir = Path::new(&self.project_path);
        if !project_dir.exists() {
            return Err(self.fail(
                reporter,
                format!(
                    "Project path '{}' does not exist. Check the 'source' field in .smb/config.toml.",
                    self.project_path
                ),
            ));
        }

        let manifest_path = project_dir.join("package.json");
        let manifest = std::fs::read_to_string(&manifest_path).map_err(|error| {
            self.fail(
                reporter,
                format!("Could not read {}: {error}", manifest_path.display()),
            )
        })?;
        let adapter = SvelteKitAdapter::from_package_json(&manifest).map_err(|error| {
            self.fail(
                reporter,
                format!("Could not parse {}: {error}", manifest_path.display()),
            )
        })?;

        match adapter {
            Some(SvelteKitAdapter::Static) => {}
            None => {
                return Err(self.fail(
                    reporter,
                    format!(
                        "'{}' does not depend on @sveltejs/kit. For a plain Svelte + Vite app, use kind = \"vite-spa\".",
                        self.project_path
                    ),
                ))
            }
            Some(SvelteKitAdapter::Missing) => {
                return Err(self.fail(
                    reporter,
                    "No SvelteKit adapter found. Add @sveltejs/adapter-static and use it in svelte.config.js.".to_string(),
                ))
            }
            Some(SvelteKitAdapter::Other(name)) => {
                return Err(self.fail(
                    reporter,
                    format!(
                        "{name} is not supported yet. smbCloud deploys SvelteKit with @sveltejs/adapter-static; switch svelte.config.js to it."
                    ),
                ))
            }
        }

        self.run(reporter, &["install"])?;
        self.run(reporter, &["run", "build"])?;

        let source_dir = project_dir.join(&self.output_dir);
        if !has_html_entry(&source_dir) {
            return Err(self.fail(
                reporter,
                format!(
                    "{} has no HTML entry point. Prerender at least one page, or set `fallback` in adapter-static for a single-page app.",
                    source_dir.display()
                ),
            ));
        }

        reporter.step_done("Build complete.");
        Ok(BuildArtifact { source_dir })
    }
}

/// Whether `directory` holds at least one top-level `.html` file: a
/// prerendered `index.html` or an adapter-static fallback such as `200.html`.
fn has_html_entry(directory: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path: PathBuf = entry.path();
        path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "html")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_static_adapter_in_dev_dependencies() {
        let manifest =
            r#"{"devDependencies":{"@sveltejs/kit":"^2","@sveltejs/adapter-static":"^3"}}"#;
        assert_eq!(
            SvelteKitAdapter::from_package_json(manifest).unwrap(),
            Some(SvelteKitAdapter::Static)
        );
    }

    #[test]
    fn prefers_static_when_several_adapters_are_listed() {
        let manifest = r#"{"dependencies":{"@sveltejs/adapter-node":"^5"},"devDependencies":{"@sveltejs/kit":"^2","@sveltejs/adapter-static":"^3"}}"#;
        assert_eq!(
            SvelteKitAdapter::from_package_json(manifest).unwrap(),
            Some(SvelteKitAdapter::Static)
        );
    }

    #[test]
    fn reports_other_adapters_by_name() {
        let manifest =
            r#"{"devDependencies":{"@sveltejs/kit":"^2","@sveltejs/adapter-auto":"^3"}}"#;
        assert_eq!(
            SvelteKitAdapter::from_package_json(manifest).unwrap(),
            Some(SvelteKitAdapter::Other(
                "@sveltejs/adapter-auto".to_string()
            ))
        );
    }

    #[test]
    fn reports_missing_adapter() {
        let manifest = r#"{"devDependencies":{"@sveltejs/kit":"^2"}}"#;
        assert_eq!(
            SvelteKitAdapter::from_package_json(manifest).unwrap(),
            Some(SvelteKitAdapter::Missing)
        );
    }

    #[test]
    fn plain_svelte_is_not_sveltekit() {
        let manifest = r#"{"devDependencies":{"svelte":"^5","vite":"^5"}}"#;
        assert_eq!(SvelteKitAdapter::from_package_json(manifest).unwrap(), None);
    }

    #[test]
    fn invalid_manifest_is_an_error() {
        assert!(SvelteKitAdapter::from_package_json("{").is_err());
    }

    #[test]
    fn html_entry_accepts_spa_fallback() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("200.html"), "<html></html>").unwrap();
        assert!(has_html_entry(directory.path()));
    }

    #[test]
    fn html_entry_rejects_missing_or_asset_only_output() {
        let directory = tempfile::tempdir().unwrap();
        assert!(!has_html_entry(&directory.path().join("build")));
        std::fs::create_dir(directory.path().join("_app")).unwrap();
        std::fs::write(directory.path().join("robots.txt"), "").unwrap();
        assert!(!has_html_entry(directory.path()));
    }
}
