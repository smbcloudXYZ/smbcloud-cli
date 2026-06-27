use crate::ar_date_format;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tsync::tsync;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthApp {
    pub id: String,
    pub secret: Option<String>,
    pub name: String,
    #[serde(with = "ar_date_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ar_date_format")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthAppCreate {
    pub name: String,
    pub description: String,
}

/// A public OAuth client registered against an AuthApp for the hosted
/// Authorization Code + PKCE flow.
///
/// These are public clients: no client secret is issued, so security rests on
/// PKCE plus the `redirect_uris` allowlist. `redirect_uris` is a newline-
/// separated list. `client_id` is the public identifier (prefixed `auc_`) sent
/// in the `/authorize` request.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct AuthAppClient {
    pub id: i64,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: String,
    pub confidential: bool,
    pub auth_app_id: String,
    #[serde(with = "ar_date_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ar_date_format")]
    pub updated_at: DateTime<Utc>,
}

/// Request body for registering a new public OAuth client on an AuthApp.
/// `redirect_uris` is a newline-separated allowlist; each entry must be https,
/// a loopback http URL, or a custom scheme (native apps).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct AuthAppClientCreate {
    pub name: String,
    pub redirect_uris: String,
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
