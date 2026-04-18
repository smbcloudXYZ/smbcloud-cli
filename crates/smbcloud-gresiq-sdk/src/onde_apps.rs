//! Apps and models management for Onde Inference.
//!
//! Each function opens its own `reqwest::Client`. These are low-frequency
//! management calls, so there's no benefit to a shared pool.
//!
//! Every request needs two things: the Onde app's client credentials as
//! query params, and the user's bearer token as an Authorization header.
//!
//! ```text
//! {protocol}://{host}/v1/client/gresiq/{path}
//!     ?client_id={app_id}&client_secret={app_secret}
//! Authorization: Bearer {access_token}
//! ```

use crate::error::GresiqError;
use serde::{Deserialize, Serialize};
use smbcloud_network::environment::Environment;

/// An app registered to the user's account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OndeApp {
    pub id: String,
    pub name: String,
    pub status: Option<String>,
    pub app_secret: Option<String>,
    pub current_model_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// A model from the Onde catalog, assignable to an [`OndeApp`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OndeModel {
    pub id: String,
    pub name: Option<String>,
    pub hf_repo_id: Option<String>,
    pub gguf_file: Option<String>,
    pub family: Option<String>,
    pub parameter_class: Option<String>,
    pub format: Option<String>,
    pub approx_size_bytes: Option<i64>,
    pub description: Option<String>,
}

// The models endpoint wraps its array: { "models": [...] }.
#[derive(Deserialize)]
struct ModelsEnvelope {
    models: Vec<OndeModel>,
}

// POST /apps body shape: { "gresiq_app": { "name": "..." } }
#[derive(Serialize)]
struct CreateAppBody<'a> {
    gresiq_app: CreateAppParams<'a>,
}

#[derive(Serialize)]
struct CreateAppParams<'a> {
    name: &'a str,
}

fn endpoint(environment: &Environment, path: &str, app_id: &str, app_secret: &str) -> String {
    format!(
        "{}://{}/v1/client/gresiq/{}?client_id={}&client_secret={}",
        environment.api_protocol(),
        environment.api_host(),
        path,
        app_id,
        app_secret,
    )
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

// Returns the response on 2xx. On anything else, reads the body as text
// before returning so callers don't have to think about it.
async fn check(response: reqwest::Response) -> Result<reqwest::Response, GresiqError> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status().as_u16();
    let message = response
        .text()
        .await
        .unwrap_or_else(|_| "unreadable response body".to_string());
    Err(GresiqError::Api { status, message })
}

/// Fetch all apps for the authenticated user.
///
/// `GET /v1/client/gresiq/apps`
pub async fn list_apps(
    environment: &Environment,
    app_id: &str,
    app_secret: &str,
    access_token: &str,
) -> Result<Vec<OndeApp>, GresiqError> {
    let url = endpoint(environment, "apps", app_id, app_secret);
    let response = reqwest::Client::new()
        .get(&url)
        .header("Authorization", bearer(access_token))
        .header("Content-Type", "application/json")
        .send()
        .await?;
    Ok(check(response).await?.json::<Vec<OndeApp>>().await?)
}

/// Create a new app under the authenticated user's account.
///
/// `POST /v1/client/gresiq/apps` â body: `{ "gresiq_app": { "name": "..." } }`
pub async fn create_app(
    environment: &Environment,
    app_id: &str,
    app_secret: &str,
    access_token: &str,
    name: &str,
) -> Result<OndeApp, GresiqError> {
    let url = endpoint(environment, "apps", app_id, app_secret);
    let body = CreateAppBody {
        gresiq_app: CreateAppParams { name },
    };
    let response = reqwest::Client::new()
        .post(&url)
        .header("Authorization", bearer(access_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;
    Ok(check(response).await?.json::<OndeApp>().await?)
}

/// Assign a catalog model to an app. Creates the record if none exists yet.
///
/// `PATCH /v1/client/gresiq/apps/{onde_app_id}/model` â body: `{ "model_id": "..." }`
pub async fn assign_model(
    environment: &Environment,
    app_id: &str,
    app_secret: &str,
    access_token: &str,
    onde_app_id: &str,
    model_id: &str,
) -> Result<(), GresiqError> {
    let path = format!("apps/{}/model", onde_app_id);
    let url = endpoint(environment, &path, app_id, app_secret);
    let body = serde_json::json!({ "model_id": model_id });
    let response = reqwest::Client::new()
        .patch(&url)
        .header("Authorization", bearer(access_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;
    check(response).await?;
    Ok(())
}

/// Fetch all models in the Onde catalog.
///
/// `GET /v1/client/gresiq/models` â response: `{ "models": [...] }`
pub async fn list_models(
    environment: &Environment,
    app_id: &str,
    app_secret: &str,
    access_token: &str,
) -> Result<Vec<OndeModel>, GresiqError> {
    let url = endpoint(environment, "models", app_id, app_secret);
    let response = reqwest::Client::new()
        .get(&url)
        .header("Authorization", bearer(access_token))
        .header("Content-Type", "application/json")
        .send()
        .await?;
    Ok(check(response).await?.json::<ModelsEnvelope>().await?.models)
}


/// Rename an existing app.
///
/// `PATCH /v1/client/gresiq/apps/{onde_app_id}` — body: `{ "gresiq_app": { "name": "..." } }`
pub async fn rename_app(
    environment: &Environment,
    app_id: &str,
    app_secret: &str,
    access_token: &str,
    onde_app_id: &str,
    new_name: &str,
) -> Result<OndeApp, GresiqError> {
    let path = format!("apps/{}", onde_app_id);
    let url = endpoint(environment, &path, app_id, app_secret);
    let body = CreateAppBody {
        gresiq_app: CreateAppParams { name: new_name },
    };
    let response = reqwest::Client::new()
        .patch(&url)
        .header("Authorization", bearer(access_token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;
    Ok(check(response).await?.json::<OndeApp>().await?)
}
