use crate::cloud_auth::{
    request::request_empty,
    url_builder::{build_auth_app_url, build_auth_apps_url},
};
use reqwest::Client;
use serde::Serialize;
use smbcloud_model::{
    app_auth::{AuthApp, AuthAppCreate, AuthAppUpdate},
    error_codes::ErrorResponse,
};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient};

#[derive(Serialize)]
struct AuthAppEnvelope<T> {
    auth_app: T,
}

pub async fn get_auth_apps(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    project_id: Option<String>,
) -> Result<Vec<AuthApp>, ErrorResponse> {
    let builder = Client::new()
        .get(build_auth_apps_url(env, client, project_id.as_deref()))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_auth_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    auth_app_id: String,
) -> Result<AuthApp, ErrorResponse> {
    let builder = Client::new()
        .get(build_auth_app_url(env, client, &auth_app_id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn create_auth_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    auth_app: AuthAppCreate,
) -> Result<AuthApp, ErrorResponse> {
    let builder = Client::new()
        .post(build_auth_apps_url(env, client, None))
        .json(&AuthAppEnvelope { auth_app })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn update_auth_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    auth_app_id: String,
    auth_app: AuthAppUpdate,
) -> Result<AuthApp, ErrorResponse> {
    let builder = Client::new()
        .put(build_auth_app_url(env, client, &auth_app_id))
        .json(&AuthAppEnvelope { auth_app })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn delete_auth_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    auth_app_id: String,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_auth_app_url(env, client, &auth_app_id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request_empty(builder).await
}
