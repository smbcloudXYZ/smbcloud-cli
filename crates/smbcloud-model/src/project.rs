use {
    crate::{app_auth::AuthApp, ar_date_format, runner::Runner},
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    serde_repr::{Deserialize_repr, Serialize_repr},
    std::fmt::Display,
    tsync::tsync,
};

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

#[derive(Deserialize, Debug, Serialize)]
pub struct Config {
    pub current_project: Option<Project>,
    pub current_auth_app: Option<AuthApp>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub runner: Runner,
    /// Defaults to `Git` when absent (older API responses won't include the field).
    #[serde(default)]
    pub deployment_method: DeploymentMethod,
    pub path: Option<String>,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Deployment kind, e.g. "vite-spa". Absent for server-side runners.
    pub kind: Option<String>,
    /// Local source directory to build from, e.g. "frontend/connected-devices".
    /// Used by vite-spa deploys as the working directory for the build step.
    /// Distinct from `path`, which is the remote destination on the server.
    pub source: Option<String>,
    /// Build output directory relative to `source`, e.g. "dist".
    pub output: Option<String>,
    /// Package manager to use for the build step, e.g. "pnpm".
    pub package_manager: Option<String>,
    /// PM2 process name to restart after a nextjs-ssr deploy, e.g. "my-app".
    /// Matches the name passed to `pm2 start` on the server.
    pub pm2_app: Option<String>,
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
