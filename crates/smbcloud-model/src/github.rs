use {
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    std::fmt::Display,
    tsync::tsync,
};

/// An installation of the smbCloud GitHub App on a user or organization
/// account, as reported by the smbCloud API.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct GithubInstallation {
    /// GitHub installation id.
    pub id: i64,
    pub account_login: String,
    /// "User" or "Organization".
    pub account_type: String,
    /// "all" or "selected".
    pub repository_selection: String,
    pub created_at: DateTime<Utc>,
}

impl Display for GithubInstallation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.account_login, self.account_type)
    }
}

/// A GitHub repository accessible through an installation.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct GithubRepository {
    /// GitHub repository id.
    pub id: i64,
    pub full_name: String,
    pub default_branch: String,
    pub private: bool,
}

impl Display for GithubRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]", self.full_name, self.default_branch)
    }
}

/// A link between a DeployRepo and a GitHub repository. Pushes to
/// `production_branch` trigger a deploy server-side.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct GithubConnection {
    pub id: i64,
    pub deploy_repo_id: i64,
    pub github_installation_id: i64,
    pub github_repo_full_name: String,
    pub production_branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Payload for connecting a GitHub repository to a DeployRepo.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct GithubConnectionCreate {
    pub github_installation_id: i64,
    pub github_repo_full_name: String,
    /// Omit to let the server default to the repository's default branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_branch: Option<String>,
}

/// Connection state of a DeployRepo, always returned with 200 so the CLI
/// can distinguish "not connected" from request errors.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[tsync]
pub struct GithubConnectionStatus {
    pub connected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<GithubConnection>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_github_connection_create_omits_missing_branch() {
        let create = GithubConnectionCreate {
            github_installation_id: 42,
            github_repo_full_name: "octocat/hello-world".to_owned(),
            production_branch: None,
        };
        let value = serde_json::to_value(&create).unwrap();
        assert_eq!(value["github_installation_id"], json!(42));
        assert_eq!(value["github_repo_full_name"], json!("octocat/hello-world"));
        assert!(value.get("production_branch").is_none());
    }

    #[test]
    fn test_github_connection_create_includes_branch() {
        let create = GithubConnectionCreate {
            github_installation_id: 42,
            github_repo_full_name: "octocat/hello-world".to_owned(),
            production_branch: Some("main".to_owned()),
        };
        let value = serde_json::to_value(&create).unwrap();
        assert_eq!(value["production_branch"], json!("main"));
    }

    #[test]
    fn test_github_connection_status_deserializes_without_connection() {
        let status: GithubConnectionStatus =
            serde_json::from_value(json!({ "connected": false })).unwrap();
        assert!(!status.connected);
        assert!(status.connection.is_none());
    }

    #[test]
    fn test_github_connection_status_deserializes_with_connection() {
        let status: GithubConnectionStatus = serde_json::from_value(json!({
            "connected": true,
            "connection": {
                "id": 1,
                "deploy_repo_id": 5,
                "github_installation_id": 42,
                "github_repo_full_name": "octocat/hello-world",
                "production_branch": "main",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            }
        }))
        .unwrap();
        assert!(status.connected);
        let connection = status.connection.expect("connection should be present");
        assert_eq!(connection.deploy_repo_id, 5);
        assert_eq!(connection.github_repo_full_name, "octocat/hello-world");
        assert_eq!(connection.production_branch, "main");
    }

    #[test]
    fn test_github_installation_round_trip() {
        let installation: GithubInstallation = serde_json::from_value(json!({
            "id": 42,
            "account_login": "octocat",
            "account_type": "User",
            "repository_selection": "selected",
            "created_at": "2026-01-01T00:00:00Z"
        }))
        .unwrap();
        assert_eq!(installation.to_string(), "octocat (User)");
        let value = serde_json::to_value(&installation).unwrap();
        assert_eq!(value["account_login"], json!("octocat"));
    }

    #[test]
    fn test_github_repository_round_trip() {
        let repository: GithubRepository = serde_json::from_value(json!({
            "id": 7,
            "full_name": "octocat/hello-world",
            "default_branch": "main",
            "private": true
        }))
        .unwrap();
        assert_eq!(repository.to_string(), "octocat/hello-world [main]");
        let value = serde_json::to_value(&repository).unwrap();
        assert_eq!(value["private"], json!(true));
    }
}
