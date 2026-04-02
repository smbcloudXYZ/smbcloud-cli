use {
    crate::client_credentials::{ClientCredentials, base_url_builder as tenant_base_url_builder},
    reqwest::Client,
    smbcloud_model::error_codes::ErrorResponse,
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::PATH_USERS, smb_base_url_builder, smb_client::SmbClient},
};

pub async fn remove(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: &str,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_smb_signup_url(env, client))
        .header("Authorization", access_token)
        .header("User-agent", client.0.id());
    request(builder).await
}

pub async fn remove_with_client(
    env: Environment,
    client: ClientCredentials<'_>,
    access_token: &str,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_remove_url_with_client(env, client))
        .header("Authorization", access_token)
        .header("User-agent", client.app_id);
    request(builder).await
}

fn build_smb_signup_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_USERS);
    url_builder.build()
}

fn build_remove_url_with_client(env: Environment, client: ClientCredentials<'_>) -> String {
    let mut url_builder = tenant_base_url_builder(env, client);
    url_builder.add_route("v1/client/me");
    url_builder.build()
}
