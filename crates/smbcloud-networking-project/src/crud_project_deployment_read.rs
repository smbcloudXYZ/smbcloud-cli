use crate::{build_project_deployment, build_project_deployment_index};
use anyhow::{anyhow, Result};
use reqwest::Client;
use smbcloud_model::project::Deployment;
use smbcloud_networking::{environment::Environment, get_smb_token};

pub async fn list_deployments(
    env: Environment,
    project_id: i32,
) -> Result<Vec<Deployment>> {
    let token = get_smb_token(env).await?;

    let response = Client::new()
        .get(build_project_deployment_index(env, project_id.to_string()))
        .header("Authorization", token)
        .header("User-agent", "smbcloud-cli")
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let deployments: Vec<Deployment> = response.json().await?;
            Ok(deployments)
        }
        _ => Err(anyhow!("Failed to list deployments.")),
    }
}

pub async fn get_deployment_detail(
    env: Environment,
    project_id: i32,
    deployment_id: i32,
) -> Result<Deployment> {
    let token = get_smb_token(env).await?;

    let url = build_project_deployment(env, project_id.to_string(), deployment_id.to_string());

    let response = Client::new()
        .get(url)
        .header("Authorization", token)
        .header("User-agent", "smbcloud-cli")
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let deployment: Deployment = response.json().await?;
            Ok(deployment)
        }
        _ => Err(anyhow!("Failed to get deployment detail.")),
    }
}