use {
    crate::{
        ar_date_format,
        project::{DeploymentMethod, DeploymentStatus},
        runner::Runner,
    },
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    serde_repr::{Deserialize_repr, Serialize_repr},
    std::fmt::Display,
    tsync::tsync,
};

/// Whether this app is a web application or a Tauri cross-platform app.
#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
#[tsync]
pub enum AppType {
    /// Web application (SPA, SSR, static site). All legacy Projects map here.
    #[default]
    Web = 0,
    /// Cross-platform desktop/mobile application built with Tauri.
    Tauri = 1,
}

impl Display for AppType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppType::Web => write!(f, "Web"),
            AppType::Tauri => write!(f, "Tauri"),
        }
    }
}

/// A deployable frontend application on the smbCloud platform.
///
/// A `FrontendApp` is the unit that actually ships.
///
/// - `Project` is the umbrella workspace
/// - `DeployRepo` is the git repository or monorepo root
/// - `FrontendApp` is the deployable app inside that repo
///
/// A `FrontendApp` belongs to a `Tenant`, is associated with an owner workspace,
/// and may optionally point at a `DeployRepo` plus a repo-relative `source_path`
/// for monorepo deployments.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct DeployRepo {
    pub id: i64,
    pub name: String,
    pub repository: String,
    pub root_path: String,
    pub repo_kind: Option<String>,
    pub runner: Option<String>,
    pub deployment_method: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct FrontendApp {
    pub id: String,
    pub name: String,
    pub app_type: AppType,
    pub runner: Runner,
    #[serde(default)]
    pub deployment_method: DeploymentMethod,
    pub project_id: i32,
    pub tenant_id: i32,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub deploy_repo_id: Option<i64>,
    pub source_path: Option<String>,
    pub deploy_repo: Option<DeployRepo>,
    pub project_ids: Vec<i32>,

    // ── CLI-local deployment config fields ───────────────────────────────────
    // These are not persisted in the database; they are stored in the local
    // .smbcloud config file alongside the FrontendApp record.
    /// Deployment kind, e.g. "vite-spa". Absent for server-side runners.
    #[serde(default)]
    pub kind: Option<String>,
    /// Local source directory to build from, e.g. "frontend/my-app".
    #[serde(default)]
    pub source: Option<String>,
    /// Build output directory relative to `source`, e.g. "dist".
    #[serde(default)]
    pub output: Option<String>,
    /// Package manager to use for the build step, e.g. "pnpm".
    #[serde(default)]
    pub package_manager: Option<String>,
    /// PM2 process name to restart after a nextjs-ssr deploy.
    #[serde(default)]
    pub pm2_app: Option<String>,
    /// Path to a shared lib directory to rsync before deploying.
    #[serde(default)]
    pub shared_lib: Option<String>,
    /// SSH command to run on the server after rsyncing the shared lib.
    #[serde(default)]
    pub compile_cmd: Option<String>,
    /// Remote destination path on the server.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub remote_path: Option<String>,
    #[serde(default)]
    pub output_path: Option<String>,
    #[serde(default)]
    pub build_command: Option<String>,
    #[serde(default)]
    pub install_command: Option<String>,
    #[serde(default)]
    pub binary_name: Option<String>,
    #[serde(default)]
    pub build_target: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub shared_lib_path: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Display for FrontendApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {}, Name: {}, Type: {}",
            self.id, self.name, self.app_type
        )
    }
}

/// Payload for creating a new FrontendApp via the API.
#[derive(Serialize, Debug, Deserialize, Clone)]
#[tsync]
pub struct FrontendAppCreate {
    pub name: String,
    pub project_id: i32,
    pub app_type: AppType,
    pub runner: Runner,
    #[serde(default)]
    pub deployment_method: DeploymentMethod,
    pub repository: Option<String>,
    pub description: Option<String>,
}

/// A deployment record tied to a FrontendApp.
#[derive(Deserialize, Serialize, Debug)]
#[tsync]
pub struct FrontendAppDeployment {
    pub id: i32,
    pub frontend_app_id: String,
    pub commit_hash: String,
    pub status: DeploymentStatus,
    #[serde(with = "ar_date_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ar_date_format")]
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_frontend_app_create_serialization() {
        let create = FrontendAppCreate {
            name: "my-app".to_owned(),
            project_id: 1,
            app_type: AppType::Web,
            runner: Runner::NodeJs,
            deployment_method: DeploymentMethod::Git,
            repository: Some("my-repo".to_owned()),
            description: None,
        };
        let value = serde_json::to_value(&create).unwrap();
        assert_eq!(value["app_type"], json!(0));
        assert_eq!(value["runner"], json!(0));
        assert_eq!(value["deployment_method"], json!(0));
    }

    #[test]
    fn test_app_type_display() {
        assert_eq!(AppType::Web.to_string(), "Web");
        assert_eq!(AppType::Tauri.to_string(), "Tauri");
    }
}
