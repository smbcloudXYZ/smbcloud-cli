use crate::url_builder::build_project_url_with_id;
use reqwest::Client;
use smbcloud_model::{error_codes::ErrorResponse, project::Project};
use smbcloud_networking::{constants::SMB_USER_AGENT, environment::Environment, network::request};

pub async fn update_project_description(
    env: Environment,
    access_token: String,
    project_id: String,
    new_description: &str,
) -> Result<Project, ErrorResponse> {
    // PATCH is correct for partial update of description
    let url = build_project_url_with_id(env, project_id.to_string());
    let builder = Client::new()
        .patch(url)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT)
        .json(&serde_json::json!({ "description": new_description }));
    request(builder).await
}
