use {
    crate::client_credentials::{ClientCredentials, base_url_builder as tenant_base_url_builder},
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        login::{AccountStatus, LoginParams, UserParam},
    },
    smbcloud_network::{environment::Environment, network::request_login},
};

pub async fn login_with_client(
    env: Environment,
    client: ClientCredentials<'_>,
    username: String,
    password: String,
) -> Result<AccountStatus, ErrorResponse> {
    let login_params = LoginParams {
        user: UserParam {
            email: username,
            password,
        },
    };
    let builder = Client::new()
        .post(build_login_url(env, client))
        .json(&login_params)
        .header("User-agent", client.app_id);
    request_login(builder).await
}

fn build_login_url(env: Environment, client: ClientCredentials<'_>) -> String {
    let mut url_builder = tenant_base_url_builder(env, client);
    url_builder.add_route("v1/client/users/sign_in");
    url_builder.build()
}
