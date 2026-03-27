use {
    crate::client_credentials::{base_url_builder as tenant_base_url_builder, ClientCredentials},
    reqwest::{Client, StatusCode},
    smbcloud_model::error_codes::{ErrorCode, ErrorResponse},
    smbcloud_network::environment::Environment,
    smbcloud_networking::{
        constants::PATH_USERS_SIGN_OUT, smb_base_url_builder, smb_client::SmbClient,
    },
};

pub async fn logout(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
) -> Result<(), ErrorResponse> {
    let response = match Client::new()
        .delete(build_smb_logout_url(env, client))
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

pub async fn logout_with_client(
    env: Environment,
    client: ClientCredentials<'_>,
    access_token: String,
) -> Result<(), ErrorResponse> {
    let response = match Client::new()
        .delete(build_logout_url_with_client(env, client))
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

fn build_smb_logout_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_USERS_SIGN_OUT);
    url_builder.build()
}

fn build_logout_url_with_client(env: Environment, client: ClientCredentials<'_>) -> String {
    let mut url_builder = tenant_base_url_builder(env, client);
    url_builder.add_route("v1/client/users/sign_out");
    url_builder.build()
}
