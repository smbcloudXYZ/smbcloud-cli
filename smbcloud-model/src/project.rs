use crate::{app_auth::AuthApp, ar_date_format};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct Config {
    pub current_project: Option<Project>,
    pub current_auth_app: Option<AuthApp>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub description: String,
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
