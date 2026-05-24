use {
    crate::url_builder::build_frontend_app_update_deploy_config_url,
    reqwest::Client,
    smbcloud_model::{
        deploy_config::DeployConfig, deploy_config_update::DeployConfigUpdate,
        error_codes::ErrorResponse,
    },
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient},
};

pub async fn update_deploy_config(
    environment: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    frontend_app_id: &str,
    payload: &DeployConfigUpdate,
) -> Result<DeployConfig, ErrorResponse> {
    let url = build_frontend_app_update_deploy_config_url(environment, client, frontend_app_id);
    let builder = Client::new()
        .patch(url)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT)
        .json(payload);
    request(builder).await
}
