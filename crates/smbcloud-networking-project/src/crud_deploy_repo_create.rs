use {
    crate::url_builder::build_deploy_repos_url,
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        frontend_app::{DeployRepo, DeployRepoCreate},
    },
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient},
};

pub async fn create_deploy_repo(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    payload: DeployRepoCreate,
) -> Result<DeployRepo, ErrorResponse> {
    let builder = Client::new()
        .post(build_deploy_repos_url(env, client))
        .json(&payload)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
