use crate::client_credentials::GresiqCredentials;
use crate::error::GresiqError;
use serde::Serialize;
use smbcloud_network::environment::Environment;
use std::collections::HashMap;

/// Talks to the smbCloud GresIQ REST gateway.
///
/// Cheap to clone — the inner `reqwest::Client` is `Arc`-backed.
/// Build one at startup and clone it wherever you need it.
///
/// # Authentication
///
/// Every request carries two headers from the GresIQ credentials:
/// `X-Gresiq-Api-Key` and `X-Gresiq-Api-Secret`. Get these from the
/// GresIQ console after registering a database.
///
/// Additional headers can be layered on top via `with_extra_headers` —
/// they ride alongside the GresIQ credentials on every subsequent request.
#[derive(Debug, Clone)]
pub struct GresiqClient {
    base_url: String,
    api_key: String,
    api_secret: String,
    extra_headers: HashMap<String, String>,
    http: reqwest::Client,
}

impl GresiqClient {
    /// Build a client from an environment and credentials.
    ///
    /// The base URL is resolved automatically from the environment:
    /// - `Environment::Dev` → `http://localhost:8088`
    /// - `Environment::Production` → `https://api.smbcloud.xyz`
    pub fn from_credentials(environment: Environment, credentials: GresiqCredentials<'_>) -> Self {
        let base_url = crate::client_credentials::base_url(&environment);
        GresiqClient {
            base_url,
            api_key: credentials.api_key.to_string(),
            api_secret: credentials.api_secret.to_string(),
            extra_headers: HashMap::new(),
            http: reqwest::Client::new(),
        }
    }

    /// Attach additional headers sent on every request alongside the GresIQ
    /// credentials. Replaces any previously set extra headers.
    ///
    /// Use this for secondary auth layers so the gateway can identify which
    /// SDK client is writing, on top of which GresIQ app owns the database.
    pub fn with_extra_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.extra_headers = headers;
        self
    }

    /// POST a record into a GresIQ-managed table.
    ///
    /// `table` is the short, un-prefixed name from the REST path —
    /// e.g. `"pulse/model_loaded"` or `"pulse_inference_events"`.
    /// The gateway resolves the tenant prefix from the api_key.
    ///
    /// Returns `Err` on network failure or a non-2xx response. The caller
    /// is responsible for deciding whether to retry, log, or ignore.
    pub async fn insert<T: Serialize>(&self, table: &str, record: &T) -> Result<(), GresiqError> {
        let url = format!("{}/gresiq/v1/{}", self.base_url, table);
        let body = serde_json::json!({ "record": record });

        let mut builder = self
            .http
            .post(&url)
            .header("X-Gresiq-Api-Key", &self.api_key)
            .header("X-Gresiq-Api-Secret", &self.api_secret)
            .json(&body);

        for (key, value) in &self.extra_headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        let response = builder.send().await?;

        if response.status().is_success() {
            log::debug!("gresiq: {} inserted ok", table);
            return Ok(());
        }

        let status = response.status().as_u16();
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "unreadable response body".to_string());

        Err(GresiqError::Api { status, message })
    }
}
