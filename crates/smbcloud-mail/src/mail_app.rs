use crate::{
    request::request_empty,
    url_builder::{build_mail_app_url, build_mail_apps_url},
};
use reqwest::Client;
use serde::Serialize;
use smbcloud_model::{
    error_codes::ErrorResponse,
    mail::{MailApp, MailAppCreate, MailAppUpdate},
};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient};

#[derive(Serialize)]
struct MailAppEnvelope<T> {
    mail_app: T,
}

pub async fn get_mail_apps(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    project_id: Option<String>,
) -> Result<Vec<MailApp>, ErrorResponse> {
    let builder = Client::new()
        .get(build_mail_apps_url(env, client, project_id.as_deref()))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_mail_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
) -> Result<MailApp, ErrorResponse> {
    let builder = Client::new()
        .get(build_mail_app_url(env, client, &mail_app_id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn create_mail_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app: MailAppCreate,
) -> Result<MailApp, ErrorResponse> {
    let builder = Client::new()
        .post(build_mail_apps_url(env, client, None))
        .json(&MailAppEnvelope { mail_app })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn update_mail_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
    mail_app: MailAppUpdate,
) -> Result<MailApp, ErrorResponse> {
    let builder = Client::new()
        .put(build_mail_app_url(env, client, &mail_app_id))
        .json(&MailAppEnvelope { mail_app })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn delete_mail_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_mail_app_url(env, client, &mail_app_id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request_empty(builder).await
}
