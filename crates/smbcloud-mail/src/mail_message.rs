use crate::url_builder::{build_mail_message_url, build_mail_messages_url};
use reqwest::Client;
use smbcloud_model::{error_codes::ErrorResponse, mail::MailMessage};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient};

pub async fn get_mail_messages(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
    inbox_id: String,
    limit: Option<u32>,
) -> Result<Vec<MailMessage>, ErrorResponse> {
    let builder = Client::new()
        .get(build_mail_messages_url(
            env,
            client,
            &mail_app_id,
            &inbox_id,
            limit,
        ))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_mail_message(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    mail_app_id: String,
    inbox_id: String,
    message_id: String,
) -> Result<MailMessage, ErrorResponse> {
    let builder = Client::new()
        .get(build_mail_message_url(
            env,
            client,
            &mail_app_id,
            &inbox_id,
            &message_id,
        ))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
