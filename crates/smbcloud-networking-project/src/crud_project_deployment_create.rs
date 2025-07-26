use crate::url_builder::build_project_deployment_index;
use anyhow::Result;
use network::environment::Environment;
use network::network::request;
use reqwest::Client;
use smbcloud_model::{
    error_codes::ErrorResponse,
    project::{Deployment, DeploymentPayload},
};
use smbcloud_networking::constants::SMB_USER_AGENT;

pub async fn create_deployment(
    env: Environment,
    access_token: &str,
    project_id: i32,
    payload: DeploymentPayload,
) -> Result<Deployment, ErrorResponse> {
    let builder = Client::new()
        .post(build_project_deployment_index(env, project_id.to_string()))
        .json(&payload)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
