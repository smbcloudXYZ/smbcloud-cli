use {
    reqwest::Client,
    smbcloud_model::{error_codes::ErrorResponse, login::AccountStatus},
    smbcloud_network::{environment::Environment, network::request_login},
    smbcloud_networking::{
        constants::PATH_ACCOUNT_STATUS, smb_base_url_builder, smb_client::SmbClient,
    },
};

pub async fn get_account_status(
    env: Environment,
    client: (&SmbClient, &str),
    email: &str,
) -> Result<AccountStatus, ErrorResponse> {
    let builder = Client::new()
        .get(build_url(env, client, email))
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");
    request_login(builder).await
}

fn build_url(env: Environment, client: (&SmbClient, &str), email: &str) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_ACCOUNT_STATUS);
    url_builder.add_param("email", email);
    url_builder.build()
}
