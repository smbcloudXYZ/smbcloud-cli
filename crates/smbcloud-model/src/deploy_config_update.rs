use serde::{Deserialize, Serialize};

/// Payload sent to PATCH /v1/frontend_apps/:id with deploy config fields.
/// Only non-None fields are serialized to the request body.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DeployConfigUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_method: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pm2_app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pm2_env: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_lib_path: Option<String>,
}

impl DeployConfigUpdate {
    /// Returns true when no field has a value — nothing to send to the server.
    pub fn is_empty(&self) -> bool {
        self.runner.is_none()
            && self.deployment_method.is_none()
            && self.kind.is_none()
            && self.source_path.is_none()
            && self.remote_path.is_none()
            && self.package_manager.is_none()
            && self.pm2_app.is_none()
            && self.pm2_env.is_none()
            && self.port.is_none()
            && self.output_path.is_none()
            && self.build_command.is_none()
            && self.install_command.is_none()
            && self.binary_name.is_none()
            && self.build_target.is_none()
            && self.shared_lib_path.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::DeployConfigUpdate;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn detects_non_empty_pm2_env() {
        let mut pm2_env = HashMap::new();
        pm2_env.insert("APP_PUBLIC_URL".to_string(), json!("https://example.com"));

        let payload = DeployConfigUpdate {
            pm2_env: Some(pm2_env),
            ..DeployConfigUpdate::default()
        };

        assert!(!payload.is_empty());
    }
}
