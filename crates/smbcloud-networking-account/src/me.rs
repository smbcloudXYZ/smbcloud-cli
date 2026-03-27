use crate::client_credentials::{base_url_builder as tenant_base_url_builder, ClientCredentials};
use reqwest::Client;
use smbcloud_model::account::User;
use smbcloud_model::error_codes::ErrorResponse;
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::PATH_USERS_ME, smb_base_url_builder, smb_client::SmbClient};

pub async fn me(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: &str,
) -> Result<User, ErrorResponse> {
    let builder = Client::new()
        .get(build_smb_info_url(env, client))
        .header("Authorization", access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");
    request(builder).await
}

pub async fn me_with_client(
    env: Environment,
    client: ClientCredentials<'_>,
    access_token: &str,
) -> Result<User, ErrorResponse> {
    let builder = Client::new()
        .get(build_info_url_with_client(env, client))
        .header("Authorization", access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");
    request(builder).await
}

fn build_smb_info_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_USERS_ME);
    url_builder.build()
}

fn build_info_url_with_client(env: Environment, client: ClientCredentials<'_>) -> String {
    let mut url_builder = tenant_base_url_builder(env, client);
    url_builder.add_route("v1/client/me");
    url_builder.build()
}
