use {
    crate::{project::DeploymentMethod, runner::Runner},
    serde::{Deserialize, Serialize},
};

/// Server-side deploy configuration returned by the API.
///
/// The CLI fetches this from `GET /v1/frontend_apps/{id}/deploy_config`
/// and merges the populated fields into the local `.smb/config.toml`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeployConfig {
    pub id: String,
    pub name: String,
    pub runner: Runner,
    pub deployment_method: DeploymentMethod,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
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
    pub package_manager: Option<String>,
    #[serde(default)]
    pub pm2_app: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub shared_lib_path: Option<String>,
    #[serde(default)]
    pub project_id: Option<i32>,
    #[serde(default)]
    pub deploy_repo_id: Option<i64>,
    #[serde(default)]
    pub repository: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_full_deploy_config() {
        let json = r#"{
            "id": "80b309e8-c39a-45b6-9121-e616fde3cd42",
            "name": "onde-cloud",
            "runner": 0,
            "deployment_method": 1,
            "source_path": ".",
            "kind": "rust",
            "remote_path": "apps/rest-api/onde-cloud",
            "output_path": null,
            "build_command": null,
            "install_command": null,
            "binary_name": "onde-cloud",
            "build_target": "x86_64-unknown-linux-gnu",
            "package_manager": null,
            "pm2_app": null,
            "port": 8090,
            "shared_lib_path": null,
            "project_id": 50,
            "deploy_repo_id": 18,
            "repository": "test"
        }"#;

        let config: DeployConfig = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(config.id, "80b309e8-c39a-45b6-9121-e616fde3cd42");
        assert_eq!(config.kind.as_deref(), Some("rust"));
        assert_eq!(
            config.remote_path.as_deref(),
            Some("apps/rest-api/onde-cloud")
        );
        assert_eq!(config.binary_name.as_deref(), Some("onde-cloud"));
        assert_eq!(
            config.build_target.as_deref(),
            Some("x86_64-unknown-linux-gnu")
        );
        assert_eq!(config.port, Some(8090));
        assert_eq!(config.runner, Runner::NodeJs);
    }

    #[test]
    fn deserializes_minimal_deploy_config() {
        let json = r#"{
            "id": "abc-123",
            "name": "my-app",
            "runner": 4,
            "deployment_method": 0
        }"#;

        let config: DeployConfig = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(config.id, "abc-123");
        assert_eq!(config.runner, Runner::Rust);
        assert!(config.kind.is_none());
        assert!(config.remote_path.is_none());
        assert!(config.port.is_none());
    }
}
