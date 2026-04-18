//! GresIQ REST client — PostgREST-compatible HTTP interface.
//!
//! Provides a fluent query builder so application code never writes SQL.
//! Talks to the GresIQ REST gateway that smbCloud runs alongside each
//! managed database.
//!
//! # Design
//!
//! Every query is built with [`GresiqRestClient::from`], which returns a
//! [`QueryBuilder`].  Filters, ordering, and pagination are added through
//! chainable methods.  Calling a terminal mutation method (`.insert()`,
//! `.update()`, `.upsert()`, `.delete()`) switches the HTTP method.
//! [`QueryBuilder::execute`] fires the HTTP request and deserialises the
//! response.
//!
//! # Example
//!
//! ```no_run
//! use gresiq::rest::{GresiqRestClient, GresiqRestConfig, OrderDir};
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize)]
//! struct Lesson { id: i32, title_en: String }
//!
//! # async fn example() -> Result<(), gresiq::rest::GresiqRestError> {
//! let config = GresiqRestConfig {
//!     base_url: "https://main.my-app.gresiq.smbcloud.xyz".to_owned(),
//!     api_key:  "my-api-key".to_owned(),
//!     schema:   None,
//! };
//! let client = GresiqRestClient::new(config)?;
//!
//! // SELECT * FROM lessons ORDER BY sort_order ASC
//! let lessons: Vec<Lesson> = client
//!     .from("lessons")
//!     .select("*")
//!     .order("sort_order", OrderDir::Asc)
//!     .execute()
//!     .await?;
//! # Ok(()) }
//! ```

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::env;
use thiserror::Error;

// --- Error -------------------------------------------------------------------

/// Errors that can arise while using the GresIQ REST client.
#[derive(Debug, Error)]
pub enum GresiqRestError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("API error {status}: {body}")]
    Api { status: u16, body: String },
    #[error("deserialisation error: {0}")]
    Deserialize(String),
}

// --- Configuration -----------------------------------------------------------

/// Configuration for the GresIQ REST gateway.
///
/// Construct directly or load from environment variables with
/// [`GresiqRestConfig::from_env`].
#[derive(Debug, Clone)]
pub struct GresiqRestConfig {
    /// Base URL of the GresIQ REST gateway,
    /// e.g. `https://main.my-app.gresiq.smbcloud.xyz`.
    pub base_url: String,
    /// API key (Bearer token) used in the `Authorization` header.
    pub api_key: String,
    /// PostgreSQL schema to target (default: `public`).
    pub schema: Option<String>,
}

impl GresiqRestConfig {
    /// Load configuration from environment variables.
    ///
    /// | Variable          | Required | Description              |
    /// |-------------------|----------|--------------------------|
    /// | `GRESIQ_REST_URL` | yes      | Base URL of the gateway  |
    /// | `GRESIQ_API_KEY`  | yes      | Bearer API key           |
    /// | `GRESIQ_SCHEMA`   | no       | Target schema (default: `public`) |
    pub fn from_env() -> Result<Self, GresiqRestError> {
        let base_url = env::var("GRESIQ_REST_URL").map_err(|_| {
            GresiqRestError::Config(
                "missing required environment variable `GRESIQ_REST_URL`".to_owned(),
            )
        })?;
        let api_key = env::var("GRESIQ_API_KEY").map_err(|_| {
            GresiqRestError::Config(
                "missing required environment variable `GRESIQ_API_KEY`".to_owned(),
            )
        })?;
        let schema = env::var("GRESIQ_SCHEMA").ok();
        Ok(Self {
            base_url,
            api_key,
            schema,
        })
    }
}

// --- Client ------------------------------------------------------------------

/// HTTP client for the GresIQ REST gateway.
///
/// Create once and reuse; the inner [`reqwest::Client`] maintains a
/// connection pool.
#[derive(Debug, Clone)]
pub struct GresiqRestClient {
    http: reqwest::Client,
    config: GresiqRestConfig,
}

impl GresiqRestClient {
    /// Build a new client from the supplied configuration.
    pub fn new(config: GresiqRestConfig) -> Result<Self, GresiqRestError> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|error| GresiqRestError::Http(error.to_string()))?;
        Ok(Self { http, config })
    }

    /// Start a fluent query against `table`.
    pub fn from(&self, table: &str) -> QueryBuilder<'_> {
        QueryBuilder {
            client: self,
            table: table.to_owned(),
            select: "*".to_owned(),
            filters: Vec::new(),
            order: Vec::new(),
            limit: None,
            offset: None,
            operation: Operation::Select,
        }
    }

    /// Returns `"{base_url}/rest/v1/{table}"`.
    pub(crate) fn table_url(&self, table: &str) -> String {
        format!("{}/rest/v1/{}", self.config.base_url, table)
    }

    /// Attaches the standard request headers required by the GresIQ gateway.
    pub(crate) fn apply_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let schema = self.config.schema.as_deref().unwrap_or("public");
        req.header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("Accept-Profile", schema)
            .header("Content-Profile", schema)
    }
}

// --- Ordering ----------------------------------------------------------------

/// Sort direction for [`QueryBuilder::order`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDir {
    Asc,
    Desc,
}

impl OrderDir {
    /// Returns the PostgREST order string for this direction.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

// --- Operation ---------------------------------------------------------------

enum Operation {
    Select,
    Insert(Value),
    Update(Value),
    Upsert(Value),
    Delete,
}

// --- QueryBuilder ------------------------------------------------------------

/// Fluent builder for a single REST request.
///
/// Obtained from [`GresiqRestClient::from`]; call [`QueryBuilder::execute`]
/// to send the request.
pub struct QueryBuilder<'c> {
    client: &'c GresiqRestClient,
    table: String,
    /// Column list sent as the `select` query-param (default `"*"`).
    select: String,
    /// Accumulated `(column, "op.value")` filter pairs.
    filters: Vec<(String, String)>,
    /// Accumulated `"col.dir"` order segments.
    order: Vec<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    operation: Operation,
}

impl<'c> QueryBuilder<'c> {
    // --- Column selection ----------------------------------------------------

    /// Override the column list returned by the query (default: `"*"`).
    pub fn select(mut self, columns: &str) -> Self {
        self.select = columns.to_owned();
        self
    }

    // --- Filters -------------------------------------------------------------

    /// `column = value`
    pub fn eq(mut self, column: &str, value: impl ToString) -> Self {
        self.filters
            .push((column.to_owned(), format!("eq.{}", value.to_string())));
        self
    }

    /// `column != value`
    pub fn neq(mut self, column: &str, value: impl ToString) -> Self {
        self.filters
            .push((column.to_owned(), format!("neq.{}", value.to_string())));
        self
    }

    /// `column > value`
    pub fn gt(mut self, column: &str, value: impl ToString) -> Self {
        self.filters
            .push((column.to_owned(), format!("gt.{}", value.to_string())));
        self
    }

    /// `column >= value`
    pub fn gte(mut self, column: &str, value: impl ToString) -> Self {
        self.filters
            .push((column.to_owned(), format!("gte.{}", value.to_string())));
        self
    }

    /// `column < value`
    pub fn lt(mut self, column: &str, value: impl ToString) -> Self {
        self.filters
            .push((column.to_owned(), format!("lt.{}", value.to_string())));
        self
    }

    /// `column <= value`
    pub fn lte(mut self, column: &str, value: impl ToString) -> Self {
        self.filters
            .push((column.to_owned(), format!("lte.{}", value.to_string())));
        self
    }

    /// `column LIKE pattern` (case-sensitive)
    pub fn like(mut self, column: &str, pattern: &str) -> Self {
        self.filters
            .push((column.to_owned(), format!("like.{pattern}")));
        self
    }

    /// `column ILIKE pattern` (case-insensitive)
    pub fn ilike(mut self, column: &str, pattern: &str) -> Self {
        self.filters
            .push((column.to_owned(), format!("ilike.{pattern}")));
        self
    }

    /// `column IN (values...)`
    pub fn in_list(mut self, column: &str, values: &[impl ToString]) -> Self {
        let joined = values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",");
        self.filters
            .push((column.to_owned(), format!("in.({joined})")));
        self
    }

    /// `column IS NULL` when `null` is `true`; `column IS NOT NULL` otherwise.
    pub fn is_null(mut self, column: &str, null: bool) -> Self {
        let filter_value = if null {
            "is.null".to_owned()
        } else {
            "is.not.null".to_owned()
        };
        self.filters.push((column.to_owned(), filter_value));
        self
    }

    // --- Ordering, paging ----------------------------------------------------

    /// Append an `ORDER BY column dir` clause.
    pub fn order(mut self, column: &str, dir: OrderDir) -> Self {
        self.order.push(format!("{}.{}", column, dir.as_str()));
        self
    }

    /// Limit the number of rows returned.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Skip the first `n` rows.
    pub fn offset(mut self, n: usize) -> Self {
        self.offset = Some(n);
        self
    }

    // --- Mutations -----------------------------------------------------------

    /// Switch to a `POST` (INSERT) request with `rows` as the JSON body.
    pub fn insert(mut self, rows: Value) -> Self {
        self.operation = Operation::Insert(rows);
        self
    }

    /// Switch to a `POST` (UPSERT) request.  Duplicate rows are merged.
    pub fn upsert(mut self, rows: Value) -> Self {
        self.operation = Operation::Upsert(rows);
        self
    }

    /// Switch to a `PATCH` (UPDATE) request with `patch` as the JSON body.
    pub fn update(mut self, patch: Value) -> Self {
        self.operation = Operation::Update(patch);
        self
    }

    /// Switch to a `DELETE` request.
    pub fn delete(mut self) -> Self {
        self.operation = Operation::Delete;
        self
    }

    // --- Terminal ------------------------------------------------------------

    /// Send the request and deserialise the response body as `T`.
    ///
    /// - For a `SELECT` that returns rows, use `T = Vec<YourRow>`.
    /// - For mutations where you do not need the returned data, use `T = ()`.
    pub async fn execute<T: DeserializeOwned>(self) -> Result<T, GresiqRestError> {
        let QueryBuilder {
            client,
            table,
            select,
            filters,
            order,
            limit,
            offset,
            operation,
        } = self;

        let url = client.table_url(&table);

        // Determine HTTP method, Prefer header, and optional JSON body from
        // the operation variant.
        let (http_method, prefer_header, body) = match operation {
            Operation::Select => ("GET", "return=representation".to_owned(), None),
            Operation::Insert(rows) => ("POST", "return=representation".to_owned(), Some(rows)),
            Operation::Update(patch) => ("PATCH", "return=representation".to_owned(), Some(patch)),
            Operation::Upsert(rows) => (
                "POST",
                "return=representation,resolution=merge-duplicates".to_owned(),
                Some(rows),
            ),
            Operation::Delete => ("DELETE", "return=representation".to_owned(), None),
        };

        let base_request = match http_method {
            "GET" => client.http.get(&url),
            "POST" => client.http.post(&url),
            "PATCH" => client.http.patch(&url),
            "DELETE" => client.http.delete(&url),
            _ => unreachable!(),
        };

        // Apply standard headers and the Prefer header.
        let mut request = client
            .apply_headers(base_request)
            .header("Prefer", prefer_header);

        // Build query-string parameters.
        let mut params: Vec<(String, String)> = Vec::new();
        params.push(("select".to_owned(), select));
        for (column, filter_value) in filters {
            params.push((column, filter_value));
        }
        if !order.is_empty() {
            params.push(("order".to_owned(), order.join(",")));
        }
        if let Some(n) = limit {
            params.push(("limit".to_owned(), n.to_string()));
        }
        if let Some(n) = offset {
            params.push(("offset".to_owned(), n.to_string()));
        }
        request = request.query(&params);

        // Attach JSON body for mutating operations.
        if let Some(json_body) = body {
            request = request.json(&json_body);
        }

        // Send and check for HTTP-level errors.
        let response = request
            .send()
            .await
            .map_err(|error| GresiqRestError::Http(error.to_string()))?;

        let status = response.status();

        if !status.is_success() {
            let status_code = status.as_u16();
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable>".to_owned());
            return Err(GresiqRestError::Api {
                status: status_code,
                body: body_text,
            });
        }

        // Handle 204 No Content — no bytes to deserialise.
        if status == StatusCode::NO_CONTENT {
            return deserialise_empty();
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|error| GresiqRestError::Http(error.to_string()))?;

        if bytes.is_empty() {
            return deserialise_empty();
        }

        serde_json::from_slice::<T>(&bytes)
            .map_err(|error| GresiqRestError::Deserialize(error.to_string()))
    }
}

/// Try sensible fallbacks when the server sends no response body.
///
/// Works for `T = ()` (deserialises `null`) and `T = Vec<_>` (deserialises
/// `[]`). Returns [`GresiqRestError::Deserialize`] if neither succeeds.
fn deserialise_empty<T: DeserializeOwned>() -> Result<T, GresiqRestError> {
    if let Ok(value) = serde_json::from_value::<T>(Value::Null) {
        return Ok(value);
    }
    if let Ok(value) = serde_json::from_str::<T>("[]") {
        return Ok(value);
    }
    Err(GresiqRestError::Deserialize("empty response".to_owned()))
}

// --- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> GresiqRestClient {
        GresiqRestClient::new(GresiqRestConfig {
            base_url: "https://example.gresiq.smbcloud.xyz".to_owned(),
            api_key: "test-key".to_owned(),
            schema: None,
        })
        .expect("client build should not fail in tests")
    }

    #[test]
    fn table_url_has_correct_shape() {
        let client = make_client();
        assert_eq!(
            client.table_url("lessons"),
            "https://example.gresiq.smbcloud.xyz/rest/v1/lessons"
        );
    }

    #[test]
    fn order_dir_as_str() {
        assert_eq!(OrderDir::Asc.as_str(), "asc");
        assert_eq!(OrderDir::Desc.as_str(), "desc");
    }

    #[test]
    fn deserialise_empty_works_for_unit() {
        let result: Result<(), GresiqRestError> = deserialise_empty();
        assert!(result.is_ok());
    }

    #[test]
    fn deserialise_empty_works_for_vec() {
        let result: Result<Vec<String>, GresiqRestError> = deserialise_empty();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn from_env_errors_on_missing_url() {
        // Ensure neither variable is set for this test.
        unsafe {
            env::remove_var("GRESIQ_REST_URL");
            env::remove_var("GRESIQ_API_KEY");
        }
        let result = GresiqRestConfig::from_env();
        assert!(matches!(result, Err(GresiqRestError::Config(_))));
    }
}
