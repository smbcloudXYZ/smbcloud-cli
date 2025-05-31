use crate::{build_project_deployment, build_project_deployment_index, build_project_url_with_id};
use anyhow::{anyhow, Result};
use reqwest::Client;
use smbcloud_model::{error_codes::ErrorResponse, project::{Deployment, Project}};
use smbcloud_networking::{constants::SMB_USER_AGENT, environment::Environment, get_smb_token, network::request};

pub async fn list_deployments(
    env: Environment,
    access_token: String,
    project_id: i32
) -> Result<Vec<Deployment>, ErrorResponse> {
    let builder = Client::new()
        .get(build_project_deployment_index(env, project_id.to_string()))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);

    request(builder).await
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

pub async fn get_project(
    env: Environment,
    access_token: String,
    id: String
) -> Result<Project, ErrorResponse> {
    let builder = Client::new()
        .get(build_project_url_with_id(env, id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);

    request(builder).await?
}