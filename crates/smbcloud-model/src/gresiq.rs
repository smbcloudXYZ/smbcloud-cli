use serde::{Deserialize, Serialize};

/// A project's GresIQ app as the owner API returns it. Key metadata only: the
/// raw API secret and keys are never part of this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GresiqApp {
    pub id: i64,
    pub name: String,
    pub project_id: i64,
    pub tenant_id: i64,
    pub table_prefix: String,
    pub status: String,
    /// The data-plane key id. Not a secret on its own; it pairs with the API
    /// secret, which this type never holds.
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub server_key_last4: Option<String>,
    #[serde(default)]
    pub server_key_rotated_at: Option<String>,
    #[serde(default)]
    pub ingest_key_last4: Option<String>,
    #[serde(default)]
    pub ingest_key_rotated_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// The one-time credentials in a create response: `api_secret` is shown once.
#[derive(Clone, Deserialize)]
pub struct GresiqAppCredentials {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_secret: Option<String>,
}

impl std::fmt::Debug for GresiqAppCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GresiqAppCredentials")
            .field("api_key", &self.api_key)
            .field(
                "api_secret",
                &self.api_secret.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

/// A freshly rotated key (`gqs_` server key or `gqi_` ingest key), shown once.
#[derive(Clone)]
pub struct RotatedKey {
    pub key: String,
    pub last4: String,
}

impl std::fmt::Debug for RotatedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RotatedKey")
            .field("key", &"<redacted>")
            .field("last4", &self.last4)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_shows_a_secret() {
        let credentials = GresiqAppCredentials {
            api_key: Some("gq_key".into()),
            api_secret: Some("SECRET".into()),
        };
        assert!(!format!("{credentials:?}").contains("SECRET"));
        let rotated = RotatedKey {
            key: "gqs_SECRET".into(),
            last4: "CRET".into(),
        };
        assert!(!format!("{rotated:?}").contains("gqs_SECRET"));
    }

    #[test]
    fn an_app_reads_from_the_api_shape() {
        let app: GresiqApp = serde_json::from_str(
            r#"{"id":7,"name":"splitfire-ai-gresiq","project_id":38,"tenant_id":2,
                "table_prefix":"splitfire_ai_gresiq","status":"active","api_key":"gq_x",
                "server_key_last4":null,"post_databases_count":0}"#,
        )
        .unwrap();
        assert_eq!(app.project_id, 38);
        assert_eq!(app.status, "active");
    }
}
