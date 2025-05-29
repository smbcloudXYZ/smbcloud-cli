use crate::{app_auth::AuthApp, ar_date_format};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Deserialize, Debug, Serialize)]
pub struct Config {
    pub current_project: Option<Project>,
    pub current_auth_app: Option<AuthApp>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub repository: String,
    pub description: Option<String>,
    #[serde(with = "ar_date_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ar_date_format")]
    pub updated_at: DateTime<Utc>,
}
#[derive(Serialize, Debug)]
pub struct ProjectCreate {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Serialize, Debug)]
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

#[derive(Deserialize_repr, Serialize_repr, Debug)]
#[repr(u8)]
pub enum DeploymentStatus {
    Started = 0,
    Failed,
    Done,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_project_create() {
        let project_create = ProjectCreate {
            name: "test".to_owned(),
            description: "test".to_owned(),
        };
        let json = json!({
            "name": "test",
            "description": "test",
        });
        assert_eq!(serde_json::to_value(project_create).unwrap(), json);
    }
}
