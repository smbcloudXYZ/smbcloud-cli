use crate::ar_date_format;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthApp {
    pub id: String,
    pub secret: Option<String>,
    pub name: String,
    #[serde(with = "ar_date_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ar_date_format")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthAppCreate {
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_auth_app_create() {
        let auth_app_create = AuthAppCreate {
            name: "test".to_owned(),
            description: "test".to_owned(),
        };
        let json = json!({
            "name": "test",
            "description": "test",
        });
        assert_eq!(serde_json::to_value(auth_app_create).unwrap(), json);
    }
}
