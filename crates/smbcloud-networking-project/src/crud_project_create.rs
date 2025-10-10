use crate::url_builder::build_project_url;
use anyhow::Result;
use reqwest::Client;
use smbcloud_model::{
    error_codes::ErrorResponse,
    project::{Project, ProjectCreate},
};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::constants::SMB_USER_AGENT;

pub async fn create_project(
    env: Environment,
    access_token: String,
    project: ProjectCreate,
) -> Result<Project, ErrorResponse> {
    let builder = Client::new()
        .post(build_project_url(env))
        .json(&project)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
