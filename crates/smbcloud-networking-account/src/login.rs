use crate::url_builder::build_smb_login_url;
use network::environment::Environment;
use network::network::request;
use reqwest::Client;
use smbcloud_model::{
    error_codes::ErrorResponse,
    login::{LoginParams, LoginResult},
};
use smbcloud_networking::constants::SMB_USER_AGENT;

pub async fn login(
    env: Environment,
    login_params: LoginParams,
) -> Result<LoginResult, ErrorResponse> {
    let builder = Client::new()
        .post(build_smb_login_url(env))
        .json(&login_params)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
