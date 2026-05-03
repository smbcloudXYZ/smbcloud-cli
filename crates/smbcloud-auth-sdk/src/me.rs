use {
    crate::client_credentials::{ClientCredentials, base_url_builder as tenant_base_url_builder},
    reqwest::Client,
    smbcloud_model::{account::User, error_codes::ErrorResponse},
    smbcloud_network::{environment::Environment, network::request},
};

pub async fn me_with_client(
    env: Environment,
    client: ClientCredentials<'_>,
    access_token: &str,
) -> Result<User, ErrorResponse> {
    let builder = Client::new()
        .get(build_me_url(env, client))
        .header("Authorization", access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");
    request(builder).await
}

fn build_me_url(env: Environment, client: ClientCredentials<'_>) -> String {
    let mut url_builder = tenant_base_url_builder(env, client);
    url_builder.add_route("v1/client/me");
    url_builder.build()
}
