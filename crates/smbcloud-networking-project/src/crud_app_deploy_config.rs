use {
    crate::url_builder::build_app_deploy_config_url,
    reqwest::Client,
    smbcloud_model::{deploy_config::DeployConfig, error_codes::ErrorResponse},
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient},
};

/// `GET /v1/apps/:app_id/deploy_config`. Unlike the `frontend_apps` route, an
/// app's own deploy token may call this, so a CI job can learn the app's
/// `publish_target` without a user session.
pub async fn get_app_deploy_config(
    environment: Environment,
    client: (&SmbClient, &str),
    access_token: &str,
    app_id: &str,
) -> Result<DeployConfig, ErrorResponse> {
    let builder = Client::new()
        .get(build_app_deploy_config_url(environment, client, app_id))
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
