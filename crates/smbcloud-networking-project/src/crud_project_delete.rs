use crate::build_project_url_with_id;
use reqwest::Client;
use smbcloud_model::error_codes::ErrorResponse;
use smbcloud_networking::{constants::SMB_USER_AGENT, environment::Environment, network::request};

pub async fn delete_project(
    env: Environment,
    access_token: String,
    id: String,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_project_url_with_id(env, id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
