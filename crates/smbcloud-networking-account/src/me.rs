use network::{environment::Environment, network::request};
use reqwest::Client;
use smbcloud_model::account::User;
use smbcloud_model::error_codes::ErrorResponse;
use smbcloud_networking::{constants::PATH_USERS_ME, smb_base_url_builder};

pub async fn me(env: Environment, access_token: &str) -> Result<User, ErrorResponse> {
    let builder = Client::new()
        .get(build_smb_info_url(env))
        .header("Authorization", access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");
    request(builder).await
}

fn build_smb_info_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_ME);
    url_builder.build()
}
