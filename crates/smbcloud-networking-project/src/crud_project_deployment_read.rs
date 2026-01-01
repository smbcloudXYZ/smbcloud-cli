use crate::url_builder::{build_project_deployment, build_project_deployment_index};
use anyhow::Result;
use reqwest::Client;
use smbcloud_model::{error_codes::ErrorResponse, project::Deployment};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient};

pub async fn get_deployments(
    env: Environment,
    client: SmbClient,
    access_token: String,
    project_id: i32,
) -> Result<Vec<Deployment>, ErrorResponse> {
    let builder = Client::new()
        .get(build_project_deployment_index(
            env,
            &client,
            project_id.to_string(),
        ))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_deployment(
    env: Environment,
    client: SmbClient,
    access_token: String,
    project_id: i32,
    deployment_id: i32,
) -> Result<Deployment, ErrorResponse> {
    let builder = Client::new()
        .get(build_project_deployment(
            env,
            &client,
            project_id.to_string(),
            deployment_id.to_string(),
        ))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
