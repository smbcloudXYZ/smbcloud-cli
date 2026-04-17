use serde::Serialize;

use crate::error::GresiqError;

/// Talks to the smbCloud GresIQ REST gateway.
///
/// Cheap to clone — the inner `reqwest::Client` is `Arc`-backed.
/// Build one at startup and clone it wherever you need it.
///
/// # Authentication
///
/// Every request carries two headers set from the fields below:
/// `X-Gresiq-Api-Key` and `X-Gresiq-Api-Secret`. Get these from the
/// GresIQ console after registering a database.
#[derive(Debug, Clone)]
pub struct GresiqClient {
    base_url:   String,
    api_key:    String,
    api_secret: String,
    http:       reqwest::Client,
}

impl GresiqClient {
    /// Build a client from explicit values.
    pub fn new(
        base_url:   impl Into<String>,
        api_key:    impl Into<String>,
        api_secret: impl Into<String>,
    ) -> Self {
        GresiqClient {
            base_url:   base_url.into(),
            api_key:    api_key.into(),
            api_secret: api_secret.into(),
            http:       reqwest::Client::new(),
        }
    }

    /// Read `GRESIQ_BASE_URL`, `GRESIQ_API_KEY`, and `GRESIQ_API_SECRET`
    /// from the environment. Returns `None` if any is missing — the caller
    /// decides what that means (skip telemetry, hard error, etc.).
    pub fn from_env() -> Option<Self> {
        let base_url   = std::env::var("GRESIQ_BASE_URL").ok()?;
        let api_key    = std::env::var("GRESIQ_API_KEY").ok()?;
        let api_secret = std::env::var("GRESIQ_API_SECRET").ok()?;
        Some(Self::new(base_url, api_key, api_secret))
    }

    /// POST a record into a GresIQ-managed table.
    ///
    /// `table` is the short, un-prefixed name from the REST path —
    /// e.g. `"pulse/model_loaded"` or `"pulse_inference_events"`.
    /// The gateway resolves the tenant prefix from the api_key.
    ///
    /// Returns `Err` on network failure or a non-2xx response. The caller
    /// is responsible for deciding whether to retry, log, or ignore.
    pub async fn insert<T: Serialize>(
        &self,
        table: &str,
        record: &T,
    ) -> Result<(), GresiqError> {
        let url  = format!("{}/gresiq/v1/{}", self.base_url, table);
        let body = serde_json::json!({ "record": record });

        let response = self
            .http
            .post(&url)
            .header("X-Gresiq-Api-Key",    &self.api_key)
            .header("X-Gresiq-Api-Secret", &self.api_secret)
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            log::debug!("gresiq: {} inserted ok", table);
            return Ok(());
        }

        let status  = response.status().as_u16();
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "unreadable response body".to_string());

        Err(GresiqError::Api { status, message })
    }
}
