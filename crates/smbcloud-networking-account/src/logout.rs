use {
    reqwest::{Client, StatusCode},
    smbcloud_model::error_codes::{ErrorCode, ErrorResponse},
    smbcloud_network::environment::Environment,
    smbcloud_networking::{constants::PATH_USERS_SIGN_OUT, smb_base_url_builder},
};

pub async fn logout(env: Environment, access_token: String) -> Result<(), ErrorResponse> {
    let response = match Client::new()
        .delete(build_smb_logout_url(env))
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

fn build_smb_logout_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_SIGN_OUT);
    url_builder.build()
}
