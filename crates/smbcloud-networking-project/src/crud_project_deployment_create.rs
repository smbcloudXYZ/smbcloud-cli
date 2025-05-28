use crate::build_project_deployment_index;
use anyhow::{anyhow, Result};
use log::debug;
use reqwest::Client;
use smbcloud_model::project::{Deployment, DeploymentPayload};
use smbcloud_networking::{environment::Environment, get_smb_token};

pub async fn create(
    env: Environment,
    project_id: i32,
    payload: DeploymentPayload,
) -> Result<Deployment> {
    // Get current token
    let token = get_smb_token(env).await?;

    debug!("Current token: {}", token);

    let response = Client::new()
        .post(build_project_deployment_index(env, project_id.to_string()))
        .json(&payload)
        .header("Authorization", token)
        .header("User-agent", "smbcloud-cli")
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let deployment: Deployment = response.json().await?;
            Ok(deployment)
        }
        _ => Err(anyhow!("Failed to fetch projects.")),
    }
}

pub async fn update(
    env: Environment,
    project_id: i32,
    deployment_id: i32,
    payload: DeploymentPayload,
) -> Result<Deployment> {
    // Get current token
    let token = get_smb_token(env).await?;

    debug!("Current token: {}", token);

    let url = format!(
        "{}/{}",
        build_project_deployment_index(env, project_id.to_string()),
        deployment_id
    );

    let response = Client::new()
        .put(url)
        .json(&payload)
        .header("Authorization", token)
        .header("User-agent", "smbcloud-cli")
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let deployment: Deployment = response.json().await?;
            Ok(deployment)
        }
        _ => Err(anyhow!("Failed to update deployment.")),
    }
}
