use crate::ar_date_format;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tsync::tsync;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthApp {
    pub id: String,
    pub secret: Option<String>,
    pub name: String,
    // Serialized as a number by the API, like `Deployment`/`MailApp`. The
    // create/update payloads take it as a String because it comes from a CLI
    // arg — the two are intentionally different types.
    pub project_id: Option<i32>,
    pub support_email: Option<String>,
    /// Whether Sign in with Apple is enabled for this auth app.
    pub apple_oauth_enabled: Option<bool>,
    /// The Services ID used as the Apple OAuth client identifier.
    pub apple_oauth_client_id: Option<String>,
    /// Whether the required Apple OAuth credentials are configured.
    pub apple_oauth_configured: Option<bool>,
    /// Whether an Apple OAuth client secret has been stored.
    pub apple_oauth_secret_present: Option<bool>,
    #[serde(with = "ar_date_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ar_date_format")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthAppCreate {
    pub name: String,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_email: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AuthAppUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_email: Option<String>,
}

impl AuthAppUpdate {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.support_email.is_none()
    }
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
    fn auth_app_apple_settings_are_optional_and_typed() -> Result<(), serde_json::Error> {
        let mut payload = json!({
            "id": "app-id",
            "name": "Example",
            "created_at": "2026-09-30T08:00:00.000+00:00",
            "updated_at": "2026-09-30T08:00:00.000+00:00"
        });
        let legacy: AuthApp = serde_json::from_value(payload.clone())?;
        assert_eq!(legacy.apple_oauth_enabled, None);
        assert_eq!(legacy.apple_oauth_client_id, None);
        assert_eq!(legacy.apple_oauth_configured, None);
        assert_eq!(legacy.apple_oauth_secret_present, None);

        payload["apple_oauth_enabled"] = json!(true);
        payload["apple_oauth_client_id"] = json!("com.example.web");
        payload["apple_oauth_configured"] = json!(true);
        payload["apple_oauth_secret_present"] = json!(true);
        let configured: AuthApp = serde_json::from_value(payload)?;
        assert_eq!(configured.apple_oauth_enabled, Some(true));
        assert_eq!(
            configured.apple_oauth_client_id.as_deref(),
            Some("com.example.web")
        );
        assert_eq!(configured.apple_oauth_configured, Some(true));
        assert_eq!(configured.apple_oauth_secret_present, Some(true));

        let serialized = serde_json::to_value(&configured)?;
        assert_eq!(serialized["apple_oauth_enabled"], json!(true));
        assert_eq!(
            serialized["apple_oauth_client_id"],
            json!("com.example.web")
        );
        assert_eq!(serialized["apple_oauth_configured"], json!(true));
        assert_eq!(serialized["apple_oauth_secret_present"], json!(true));
        Ok(())
    }
    #[test]
    fn test_auth_app_create() {
        let auth_app_create = AuthAppCreate {
            name: "test".to_owned(),
            project_id: "1".to_owned(),
            support_email: None,
        };
        let json = json!({
            "name": "test",
            "project_id": "1",
        });
        assert_eq!(serde_json::to_value(auth_app_create).unwrap(), json);
    }
}
