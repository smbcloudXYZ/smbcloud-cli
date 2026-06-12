use {
    crate::{app_auth::AuthApp, ar_date_format, runner::Runner},
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    serde_repr::{Deserialize_repr, Serialize_repr},
    std::fmt::Display,
    tsync::tsync,
};

fn default_datetime() -> DateTime<Utc> {
    DateTime::UNIX_EPOCH
}

/// How the project's files are delivered to the server.
///
/// `Git`   — the classic smbCloud flow: push to a remote git repo, the server
///           builds and restarts the process.
/// `Rsync` — files are transferred directly with rsync over SSH; no build step
///           runs on the server. Ideal for pre-built static sites or assets.
#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
#[tsync]
pub enum DeploymentMethod {
    #[default]
    Git = 0,
    Rsync = 1,
}

impl Display for DeploymentMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentMethod::Git => write!(f, "Git"),
            DeploymentMethod::Rsync => write!(f, "Rsync"),
        }
    }
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Config {
    /// Legacy project field — kept for backward compatibility during migration.
    pub current_project: Option<Project>,
    /// The active FrontendApp for CLI deploy operations.
    #[serde(default)]
    pub current_frontend_app: Option<crate::frontend_app::FrontendApp>,
    pub current_auth_app: Option<AuthApp>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct Project {
    /// Umbrella smbCloud workspace ID.
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub runner: Runner,
    /// Defaults to `Git` when absent (older API responses won't include the field).
    #[serde(default)]
    pub deployment_method: DeploymentMethod,
    pub path: Option<String>,
    pub repository: Option<String>,
    pub description: Option<String>,
    /// Deployable app ID for precise deployment tracking. Optional during the
    /// migration away from project-as-app semantics.
    pub frontend_app_id: Option<String>,
    /// Repo ID backing this deploy target. Optional until the API exposes it
    /// consistently to the CLI.
    pub deploy_repo_id: Option<i64>,
    /// Repo-relative app path for monorepo targets, e.g. "apps/web/console".
    pub source_path: Option<String>,
    #[serde(default = "default_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_datetime")]
    pub updated_at: DateTime<Utc>,
    /// Deployment kind, e.g. "vite-spa", "nextjs-ssr", or "rust".
    pub kind: Option<String>,
    /// Local source directory to build from, e.g. "frontend/connected-devices"
    /// or a Rust crate root like ".".
    /// Used by local-build deploys such as vite-spa, nextjs-ssr, and rust.
    /// Distinct from `path`, which is the remote destination on the server.
    pub source: Option<String>,
    /// Build output directory relative to `source`, e.g. "dist".
    pub output: Option<String>,
    /// Package manager to use for the build step, e.g. "pnpm".
    pub package_manager: Option<String>,
    /// PM2 process name to restart after a nextjs-ssr deploy, e.g. "my-app".
    /// Matches the name passed to `pm2 start` on the server.
    pub pm2_app: Option<String>,
    /// Environment variables to seed into the PM2 ecosystem `env_production` block.
    /// Populated from the server-side App record; not written to `.smb/config.toml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pm2_env: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Port the standalone server binds to (default: 3000). Must match nginx upstream configuration.
    #[serde(default)]
    pub port: Option<u16>,
    /// Path to a shared lib directory to rsync to the server before deploying,
    /// e.g. "lib". Used by Rails apps that depend on native gems built from
    /// monorepo-level source. Relative to the repo root.
    pub shared_lib: Option<String>,
    /// SSH command to run on the server after rsyncing the shared lib,
    /// e.g. "cd ~/lib/gems/gem_error_codes && rbenv local 3.4.2 && bundle install && bundle exec rake compile".
    pub compile_cmd: Option<String>,
    /// Install command override, e.g. "pnpm install --frozen-lockfile".
    #[serde(default)]
    pub install_command: Option<String>,
    /// Rust binary filename to upload and restart, e.g. "onde-cloud".
    /// When absent, the CLI falls back to the Cargo package name.
    pub binary_name: Option<String>,
    /// Rust target triple used for local cross-compilation before upload,
    /// e.g. "x86_64-unknown-linux-gnu".
    pub rust_target: Option<String>,
    /// Swift SDK identifier used to cross-compile a Swift/Vapor app for Linux,
    /// e.g. "x86_64-swift-linux-musl" (the Static Linux SDK). Defaults to
    /// "x86_64-swift-linux-musl" when absent. Built natively on the host with
    /// `swift build --swift-sdk <id>` — no Docker, no emulation.
    pub swift_sdk: Option<String>,
    /// Optional toolchain identifier passed via the `TOOLCHAINS` env var when
    /// cross-compiling Swift. Needed on macOS where the default `swift` is
    /// Apple's Xcode toolchain (which lacks `lld`); point it at an installed
    /// swift.org toolchain, e.g. "swift" (resolves to swift-latest) or a bundle
    /// id like "org.swift.632202605101a". Unnecessary when `swift` is already a
    /// swift.org toolchain (e.g. via swiftly).
    pub swift_toolchain: Option<String>,
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {}, Name: {}", self.id, self.name,)
    }
}
#[derive(Serialize, Debug, Deserialize, Clone)]
#[tsync]
pub struct ProjectCreate {
    pub name: String,
    pub repository: String,
    pub description: String,
    pub runner: Runner,
    #[serde(default)]
    pub deployment_method: DeploymentMethod,
}

#[derive(Deserialize, Serialize, Debug)]
#[tsync]
pub struct Deployment {
    pub id: i32,
    pub project_id: i32,
    pub frontend_app_id: Option<String>,
    pub frontend_app_name: Option<String>,
    pub commit_hash: String,
    pub status: DeploymentStatus,
    #[serde(with = "ar_date_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ar_date_format")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DeploymentPayload {
    pub commit_hash: String,
    pub status: DeploymentStatus,
    pub frontend_app_id: Option<String>,
}

#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, Copy)] // Added Clone, Copy
#[repr(u8)]
#[tsync]
pub enum DeploymentStatus {
    Started = 0,
    Failed,
    Done,
}

impl Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentStatus::Started => write!(f, "🚀"),
            DeploymentStatus::Failed => write!(f, "❌"),
            DeploymentStatus::Done => write!(f, "✅"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_project_create() {
        let project_create = ProjectCreate {
            name: "test".to_owned(),
            repository: "test".to_owned(),
            description: "test".to_owned(),
            runner: Runner::NodeJs,
            deployment_method: DeploymentMethod::Git,
        };
        let json = json!({
            "name": "test",
            "repository": "test",
            "description": "test",
            "runner": 0,
            "deployment_method": 0
        });
        assert_eq!(serde_json::to_value(project_create).unwrap(), json);
    }

    #[test]
    fn test_deployment_status_display() {
        assert_eq!(format!("{}", DeploymentStatus::Started), "🚀");
        assert_eq!(DeploymentStatus::Started.to_string(), "🚀");

        assert_eq!(format!("{}", DeploymentStatus::Failed), "❌");
        assert_eq!(DeploymentStatus::Failed.to_string(), "❌");

        assert_eq!(format!("{}", DeploymentStatus::Done), "✅");
        assert_eq!(DeploymentStatus::Done.to_string(), "✅");
    }
}
