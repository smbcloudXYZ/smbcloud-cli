use {
    reqwest::Client,
    smbcloud_model::error_codes::ErrorResponse,
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::PATH_USERS, smb_base_url_builder, smb_client::SmbClient},
};

pub async fn remove(
    env: Environment,
    client: (&SmbClient, &str),
    user_agent: String,
    access_token: &str,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_smb_signup_url(env, client))
        .header("Authorization", access_token)
        .header("User-agent", user_agent);
    request(builder).await
}

fn build_smb_signup_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_USERS);
    url_builder.build()
}
