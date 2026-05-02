use crate::{
    request::request_empty,
    url_builder::{build_mail_inbox_test_url, build_mail_inbox_url, build_mail_inboxes_url},
};
use reqwest::Client;
use serde::Serialize;
use smbcloud_model::{
    error_codes::ErrorResponse,
    mail::{
        MailInbox, MailInboxCreate, MailInboxUpdate, MailTestEmailDelivery, MailTestEmailRequest,
    },
};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient};

#[derive(Serialize)]
struct MailInboxEnvelope<T> {
    mail_inbox: T,
}

pub async fn create_mail_inbox(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
    mail_inbox: MailInboxCreate,
) -> Result<MailInbox, ErrorResponse> {
    let builder = Client::new()
        .post(build_mail_inboxes_url(env, client, &mail_app_id))
        .json(&MailInboxEnvelope { mail_inbox })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn update_mail_inbox(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
    inbox_id: String,
    mail_inbox: MailInboxUpdate,
) -> Result<MailInbox, ErrorResponse> {
    let builder = Client::new()
        .put(build_mail_inbox_url(env, client, &mail_app_id, &inbox_id))
        .json(&MailInboxEnvelope { mail_inbox })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn delete_mail_inbox(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
    inbox_id: String,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_mail_inbox_url(env, client, &mail_app_id, &inbox_id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request_empty(builder).await
}

pub async fn send_test_email(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
    inbox_id: String,
    mail_inbox: MailTestEmailRequest,
) -> Result<MailTestEmailDelivery, ErrorResponse> {
    let builder = Client::new()
        .post(build_mail_inbox_test_url(
            env,
            client,
            &mail_app_id,
            &inbox_id,
        ))
        .json(&MailInboxEnvelope { mail_inbox })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
