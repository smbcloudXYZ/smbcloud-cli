pub mod crud_project_deployment_create;
pub mod crud_project_deployment_read;

use anyhow::{anyhow, Result};
use log::debug;
use reqwest::{Client, StatusCode};
use smbcloud_model::{
    self, error_codes::ErrorResponse, project::{Project, ProjectCreate}
};
use smbcloud_networking::{constants::SMB_USER_AGENT, environment::Environment, get_smb_token, network::request, smb_base_url_builder};

pub async fn get_all(env: Environment) -> Result<Vec<Project>> {
    // Get current token
    let token = get_smb_token(env).await?;

    debug!("Current token: {}", token);

    let response = Client::new()
        .get(build_project_url(env))
        .header("Authorization", token)
        .header("User-agent", "smbcloud-cli")
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let projects: Vec<Project> = response.json().await?;
            Ok(projects)
        }
        _ => Err(anyhow!("Failed to fetch projects.")),
    }
}

pub async fn create_project(env: Environment, project: ProjectCreate) -> Result<Project> {
    // Get current token
    let token = get_smb_token(env).await?;

    let response = Client::new()
        .post(build_project_url(env))
        .json(&project)
        .header("Authorization", token)
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::CREATED => {
            let project: Project = response.json().await?;
            // println!("Project created: {project:#?}");
            Ok(project)
        }
        _ => Err(anyhow!("Failed to create a project.")),
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

pub async fn delete_project(env: Environment, id: String) -> Result<()> {
    // Get current token
    let token = get_smb_token(env).await?;

    let response = Client::new()
        .delete(build_project_url_with_id(env, id))
        .header("Authorization", token)
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => {
            debug!("Project deleted.");
            Ok(())
        }
        StatusCode::NOT_FOUND => Err(anyhow!("Failed to delete a project: project not found.")),
        _ => Err(anyhow!("Failed to delete a project: unknown error.")),
    }
}

// Private functions

fn build_project_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route("v1/projects");
    url_builder.build()
}

fn build_project_url_with_id(env: Environment, id: String) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route("v1/projects");
    url_builder.add_route(id.as_str());
    url_builder.build()
}

fn build_project_deployment_index(env: Environment, project_id: String) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route("v1/projects");
    url_builder.add_route(project_id.as_str());
    url_builder.add_route("deployment");
    url_builder.build()
}

fn build_project_deployment(env: Environment, project_id: String, id: String) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route("v1/projects");
    url_builder.add_route(project_id.as_str());
    url_builder.add_route("deployment");
    url_builder.add_route(id.as_str());
    url_builder.build()
}
