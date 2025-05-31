use crate::{build_project_deployment, build_project_deployment_index, build_project_url_with_id};
use anyhow::Result;
use reqwest::Client;
use smbcloud_model::{
    error_codes::ErrorResponse,
    project::{Deployment, Project},
};
use smbcloud_networking::{constants::SMB_USER_AGENT, environment::Environment, network::request};

pub async fn get_deployments(
    env: Environment,
    access_token: String,
    project_id: i32,
) -> Result<Vec<Deployment>, ErrorResponse> {
    let builder = Client::new()
        .get(build_project_deployment_index(env, project_id.to_string()))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_deployment(
    env: Environment,
    access_token: String,
    project_id: i32,
    deployment_id: i32,
) -> Result<Deployment, ErrorResponse> {
    let builder = Client::new()
        .get(build_project_deployment(
            env,
            project_id.to_string(),
            deployment_id.to_string(),
        ))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_project(
    env: Environment,
    access_token: String,
    id: String,
) -> Result<Project, ErrorResponse> {
    let builder: reqwest::RequestBuilder = Client::new()
        .get(build_project_url_with_id(env, id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
