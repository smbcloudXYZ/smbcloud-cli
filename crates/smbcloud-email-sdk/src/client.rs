use crate::client_credentials::EmailCredentials;
use crate::error::EmailError;
use crate::message::{EmailMessage, SendEmail};
use smbcloud_network::environment::Environment;

/// Talks to the smbCloud transactional email API.
///
/// Cheap to clone — the inner `reqwest::Client` is `Arc`-backed. Build one at
/// startup and clone it wherever you need it.
///
/// # Authentication
///
/// Every request carries `Authorization: Bearer <api_key>` from the
/// [`EmailCredentials`]. Mint a key for your Mail app in the smbCloud console;
/// sending is scoped to that app's verified domain. Reading messages
/// (`get_message`, `list_messages`) needs a key with read scope.
#[derive(Debug, Clone)]
pub struct EmailClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl EmailClient {
    /// Build a client from an environment and credentials.
    ///
    /// The base URL is resolved from the environment:
    /// - `Environment::Dev` → `http://localhost:8088`
    /// - `Environment::Production` → `https://api.smbcloud.xyz`
    pub fn from_credentials(environment: Environment, credentials: EmailCredentials<'_>) -> Self {
        EmailClient {
            base_url: crate::client_credentials::base_url(&environment),
            api_key: credentials.api_key.to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Send a transactional email: `POST /v1/email/messages`.
    ///
    /// Returns the created [`EmailMessage`] (status `Sent`). If the message
    /// carried an `idempotency_key` that was already used, the original message
    /// is returned and nothing is sent again.
    pub async fn send(&self, message: &SendEmail) -> Result<EmailMessage, EmailError> {
        let url = format!("{}/v1/email/messages", self.base_url);
        let response = self
            .authed(self.http.post(&url))
            .json(message)
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(response.json().await?);
        }
        Err(self.api_error(response).await)
    }

    /// Fetch one message by id with its delivery-event timeline:
    /// `GET /v1/email/messages/:id`. Requires a read-scope key.
    pub async fn get_message(&self, id: &str) -> Result<EmailMessage, EmailError> {
        let url = format!("{}/v1/email/messages/{}", self.base_url, id);
        let response = self.authed(self.http.get(&url)).send().await?;

        if response.status().is_success() {
            return Ok(response.json().await?);
        }
        Err(self.api_error(response).await)
    }

    /// List recent messages: `GET /v1/email/messages`. Requires a read-scope
    /// key. `status` filters by delivery status name (e.g. `"delivered"`);
    /// `limit` is clamped server-side to `1..=100`.
    pub async fn list_messages(
        &self,
        status: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<EmailMessage>, EmailError> {
        let url = format!("{}/v1/email/messages", self.base_url);

        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(status) = status {
            params.push(("status", status.to_string()));
        }
        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }

        let response = self
            .authed(self.http.get(&url))
            .query(&params)
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(response.json().await?);
        }
        Err(self.api_error(response).await)
    }

    fn authed(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.header("Authorization", format!("Bearer {}", self.api_key))
    }

    async fn api_error(&self, response: reqwest::Response) -> EmailError {
        let status = response.status().as_u16();
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "unreadable response body".to_string());
        EmailError::Api { status, message }
    }
}
