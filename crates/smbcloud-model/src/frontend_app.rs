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
/// Replaces the legacy `Project` as the primary unit of deployment.
/// A `FrontendApp` belongs to a `Tenant` directly and is associated with an
/// owner workspace (Project). It can be shared across multiple workspaces.
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
    pub project_ids: Vec<i32>,

    // ── CLI-local deployment config fields ───────────────────────────────────
    // These are not persisted in the database; they are stored in the local
    // .smbcloud config file alongside the FrontendApp record.
    /// Deployment kind, e.g. "vite-spa". Absent for server-side runners.
    pub kind: Option<String>,
    /// Local source directory to build from, e.g. "frontend/my-app".
    pub source: Option<String>,
    /// Build output directory relative to `source`, e.g. "dist".
    pub output: Option<String>,
    /// Package manager to use for the build step, e.g. "pnpm".
    pub package_manager: Option<String>,
    /// PM2 process name to restart after a nextjs-ssr deploy.
    pub pm2_app: Option<String>,
    /// Path to a shared lib directory to rsync before deploying.
    pub shared_lib: Option<String>,
    /// SSH command to run on the server after rsyncing the shared lib.
    pub compile_cmd: Option<String>,
    /// Remote destination path on the server.
    pub path: Option<String>,

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
