use {
    crate::client_credentials::{ClientCredentials, base_url_builder as tenant_base_url_builder},
    reqwest::{Client, StatusCode},
    smbcloud_model::error_codes::{ErrorCode, ErrorResponse},
    smbcloud_network::environment::Environment,
};

pub async fn logout_with_client(
    env: Environment,
    client: ClientCredentials<'_>,
    access_token: String,
) -> Result<(), ErrorResponse> {
    let response = match Client::new()
        .delete(build_logout_url(env, client))
        .header("Authorization", access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => {
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::Unknown,
                message: ErrorCode::Unknown.message(None).to_string(),
            });
        }
    };

    match response.status() {
        StatusCode::OK => Ok(()),
        _ => Err(ErrorResponse::Error {
            error_code: ErrorCode::Unauthorized,
            message: ErrorCode::Unauthorized.message(None).to_string(),
        }),
    }
}

fn build_logout_url(env: Environment, client: ClientCredentials<'_>) -> String {
    let mut url_builder = tenant_base_url_builder(env, client);
    url_builder.add_route("v1/client/users/sign_out");
    url_builder.build()
}
