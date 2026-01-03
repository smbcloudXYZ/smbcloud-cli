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

fn build_smb_info_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_USERS_ME);
    url_builder.build()
}
