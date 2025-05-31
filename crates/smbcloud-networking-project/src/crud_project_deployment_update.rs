use anyhow::Result;
use reqwest::Client;
use smbcloud_model::{
    error_codes::ErrorResponse,
    project::{Deployment, DeploymentPayload},
};
use smbcloud_networking::{environment::Environment, network::request};

use crate::build_project_deployment;

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
        .header("Authorization", access_token);
    request(builder).await
}
