pub mod crud_project_delete;
pub mod crud_project_deployment_create;
pub mod crud_project_deployment_read;
pub mod crud_project_deployment_update;
pub mod crud_project_read;

use anyhow::Result;
use reqwest::Client;
use smbcloud_model::{
    error_codes::ErrorResponse,
    project::{Project, ProjectCreate},
};
use smbcloud_networking::{environment::Environment, network::request, smb_base_url_builder};

pub async fn create_project(
    env: Environment,
    access_token: String,
    project: ProjectCreate,
) -> Result<Project, ErrorResponse> {
    let builder = Client::new()
        .post(build_project_url(env))
        .json(&project)
        .header("Authorization", access_token);
    request(builder).await
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
