use {
    crate::url_builder::build_frontend_apps_url,
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        frontend_app::{FrontendApp, FrontendAppCreate},
    },
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient},
};

pub async fn create_frontend_app(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    payload: FrontendAppCreate,
) -> Result<FrontendApp, ErrorResponse> {
    let builder = Client::new()
        .post(build_frontend_apps_url(env, client))
        .json(&payload)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
