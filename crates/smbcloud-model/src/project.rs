use {
    crate::{app_auth::AuthApp, ar_date_format},
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    serde_repr::{Deserialize_repr, Serialize_repr},
    std::fmt::Display,
    tsync::tsync,
};

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
    pub repository: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {}, Name: {}", self.id, self.name,)
    }
}
#[derive(Serialize, Debug)]
pub struct ProjectCreate {
    pub name: String,
    pub repository: String,
    pub description: String,
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
        };
        let json = json!({
            "name": "test",
            "repository": "test", // Corrected: repository should be included as per struct
            "description": "test",
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
