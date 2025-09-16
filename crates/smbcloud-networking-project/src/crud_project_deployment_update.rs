use anyhow::Result;
use reqwest::Client;
use smbcloud_model::{
    error_codes::ErrorResponse,
    project::{Deployment, DeploymentPayload},
};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::constants::SMB_USER_AGENT;

use crate::url_builder::build_project_deployment;

pub async fn update(
    env: Environment,
    access_token: String,
    project_id: i32,
    deployment_id: i32,
    status: DeploymentPayload,
) -> Result<Deployment, ErrorResponse> {
    let url = build_project_deployment(env, project_id.to_string(), deployment_id.to_string());
    let builder = Client::new()
        .put(url)
        .json(&status)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
