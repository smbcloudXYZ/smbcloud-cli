use crate::client_credentials::GresiqCredentials;
use crate::error::GresiqError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use smbcloud_network::environment::Environment;
use std::collections::HashMap;

/// Query parameters for `get_collection` against the document gateway.
///
/// `filter` is a JSON containment object (`doc @> filter`); the gateway can
/// only order by `created_at` / `updated_at`, so `order`/`dir` are limited to
/// those columns. `limit` is clamped server-side to `1..=1000`.
#[derive(Debug, Clone, Default)]
pub struct DocumentQuery {
    pub filter: Option<serde_json::Value>,
    pub order: Option<String>,
    pub dir: Option<String>,
    pub limit: Option<u32>,
}

/// One row from the document gateway's `{ documents: [...] }` envelope.
///
/// `T` is the caller's document shape — schema knowledge lives in the caller.
/// The platform metadata (`id`, `key`, timestamps) rides alongside it.
#[derive(Debug, Clone, Deserialize)]
pub struct GresiqDocument<T> {
    pub id: String,
    pub key: String,
    pub collection: String,
    pub doc: T,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
struct DocumentsEnvelope<T> {
    documents: Vec<GresiqDocument<T>>,
}

/// How a client proves itself to the gateway.
#[derive(Clone)]
enum Auth {
    /// `X-Gresiq-Api-Key` + `X-Gresiq-Api-Secret` headers.
    KeySecret { api_key: String, api_secret: String },
    /// `Authorization: Bearer gqi_...`, a write-only ingest key.
    IngestKey(String),
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Auth::KeySecret { .. } => f.write_str("Auth::KeySecret"),
            Auth::IngestKey(_) => f.write_str("Auth::IngestKey"),
        }
    }
}

/// Talks to the smbCloud GresIQ REST gateway.
///
/// Cheap to clone — the inner `reqwest::Client` is `Arc`-backed.
/// Build one at startup and clone it wherever you need it.
///
/// # Authentication
///
/// Every request carries two headers from the GresIQ credentials:
/// `X-Gresiq-Api-Key` and `X-Gresiq-Api-Secret`. Get these from the
/// GresIQ console after registering a database. A client built with
/// `from_ingest_key` sends `Authorization: Bearer` with the write-only ingest
/// key instead.
///
/// Additional headers can be layered on top via `with_extra_headers` —
/// they ride alongside the GresIQ credentials on every subsequent request.
#[derive(Debug, Clone)]
pub struct GresiqClient {
    base_url: String,
    auth: Auth,
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
            auth: Auth::KeySecret {
                api_key: credentials.api_key.to_string(),
                api_secret: credentials.api_secret.to_string(),
            },
            extra_headers: HashMap::new(),
            http: reqwest::Client::new(),
        }
    }

    /// Build a client that authenticates with a write-only `gqi_` ingest key,
    /// sent as `Authorization: Bearer`. The gateway accepts it only for
    /// telemetry writes (`insert` and `put_document` on the pulse routes and
    /// document create); reads are refused. Unlike the key and secret, it is
    /// intended to be embedded in a shipped client.
    pub fn from_ingest_key(environment: Environment, ingest_key: &str) -> Self {
        let base_url = crate::client_credentials::base_url(&environment);
        GresiqClient {
            base_url,
            auth: Auth::IngestKey(ingest_key.to_string()),
            extra_headers: HashMap::new(),
            http: reqwest::Client::new(),
        }
    }

    /// Like `from_ingest_key`, with an explicit base URL (for tests and
    /// self-hosted gateways).
    pub fn from_ingest_key_with_base_url(base_url: &str, ingest_key: &str) -> Self {
        GresiqClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth: Auth::IngestKey(ingest_key.to_string()),
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

        let response = self.authed_post(&url).json(&body).send().await?;

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

    /// Upsert (or append) a document into a collection via the M1 document
    /// gateway: `POST /gresiq/v1/collections/:collection` with body
    /// `{ key?, doc }`.
    ///
    /// With `key`: upsert on `(app, collection, key)` — the natural-key target.
    /// Without `key`: append-only insert with a server-generated UUID key.
    ///
    /// This is distinct from [`insert`](Self::insert), which targets the older
    /// semantic routes (`/gresiq/v1/<table>`); new code writing arbitrary
    /// documents should use this method.
    pub async fn upsert_document<T: Serialize>(
        &self,
        collection: &str,
        key: Option<&str>,
        doc: &T,
    ) -> Result<(), GresiqError> {
        let url = format!("{}/gresiq/v1/collections/{}", self.base_url, collection);
        let body = match key {
            Some(key) => serde_json::json!({ "key": key, "doc": doc }),
            None => serde_json::json!({ "doc": doc }),
        };

        let response = self.authed_post(&url).json(&body).send().await?;

        if response.status().is_success() {
            log::debug!("gresiq: upserted document into {}", collection);
            return Ok(());
        }

        Err(self.api_error(response).await)
    }

    /// Fetch documents from a collection via
    /// `GET /gresiq/v1/collections/:collection`, with optional containment
    /// filter, ordering, and limit (see [`DocumentQuery`]).
    ///
    /// Each returned [`GresiqDocument`] deserializes its `doc` into `T`; the
    /// caller owns the document shape.
    pub async fn get_collection<T: DeserializeOwned>(
        &self,
        collection: &str,
        query: &DocumentQuery,
    ) -> Result<Vec<GresiqDocument<T>>, GresiqError> {
        let url = format!("{}/gresiq/v1/collections/{}", self.base_url, collection);

        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(filter) = &query.filter {
            params.push(("filter", filter.to_string()));
        }
        if let Some(order) = &query.order {
            params.push(("order", order.clone()));
        }
        if let Some(dir) = &query.dir {
            params.push(("dir", dir.clone()));
        }
        if let Some(limit) = query.limit {
            params.push(("limit", limit.to_string()));
        }

        let response = self.authed_get(&url).query(&params).send().await?;

        if !response.status().is_success() {
            return Err(self.api_error(response).await);
        }

        let envelope: DocumentsEnvelope<T> = response.json().await?;
        Ok(envelope.documents)
    }

    /// Build a POST request carrying the GresIQ credentials and extra headers.
    fn authed_post(&self, url: &str) -> reqwest::RequestBuilder {
        self.with_auth(self.http.post(url))
    }

    /// Build a GET request carrying the GresIQ credentials and extra headers.
    fn authed_get(&self, url: &str) -> reqwest::RequestBuilder {
        self.with_auth(self.http.get(url))
    }

    fn with_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut builder = match &self.auth {
            Auth::KeySecret {
                api_key,
                api_secret,
            } => builder
                .header("X-Gresiq-Api-Key", api_key)
                .header("X-Gresiq-Api-Secret", api_secret),
            Auth::IngestKey(key) => builder.bearer_auth(key),
        };
        for (key, value) in &self.extra_headers {
            builder = builder.header(key.as_str(), value.as_str());
        }
        builder
    }

    /// Turn a non-2xx response into a `GresiqError::Api`, reading the body.
    async fn api_error(&self, response: reqwest::Response) -> GresiqError {
        let status = response.status().as_u16();
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "unreadable response body".to_string());
        GresiqError::Api { status, message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Accept one request, reply 201, and return the lowercased request head.
    async fn serve_once() -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let _ = socket
                .write_all(
                    b"HTTP/1.1 201 Created\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                )
                .await;
            String::from_utf8_lossy(&buf[..n]).to_lowercase()
        });
        (base, handle)
    }

    #[tokio::test]
    async fn ingest_key_is_sent_as_a_bearer_alone() {
        let (base, handle) = serve_once().await;
        let client = GresiqClient::from_ingest_key_with_base_url(&base, "gqi_abc");
        client
            .insert("pulse/model_loaded", &serde_json::json!({ "edge_id": "e" }))
            .await
            .unwrap();
        let head = handle.await.unwrap();
        assert!(head.starts_with("post /gresiq/v1/pulse/model_loaded"));
        assert!(head.contains("authorization: bearer gqi_abc"));
        assert!(!head.contains("x-gresiq-api-key"));
        assert!(!head.contains("x-gresiq-api-secret"));
    }

    #[tokio::test]
    async fn key_and_secret_still_use_the_headers() {
        let (base, handle) = serve_once().await;
        let mut client = GresiqClient::from_credentials(
            Environment::Dev,
            GresiqCredentials {
                api_key: "key",
                api_secret: "secret",
            },
        );
        client.base_url = base;
        client
            .insert("pulse/model_loaded", &serde_json::json!({}))
            .await
            .unwrap();
        let head = handle.await.unwrap();
        assert!(head.contains("x-gresiq-api-key: key"));
        assert!(head.contains("x-gresiq-api-secret: secret"));
        assert!(!head.contains("authorization:"));
    }
}
