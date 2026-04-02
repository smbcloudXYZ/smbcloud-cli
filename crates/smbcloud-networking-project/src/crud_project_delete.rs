use crate::url_builder::build_project_url_with_id;
use reqwest::Client;
use smbcloud_model::error_codes::ErrorResponse;
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient};

pub async fn delete_project(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    id: String,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_project_url_with_id(env, client, id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
