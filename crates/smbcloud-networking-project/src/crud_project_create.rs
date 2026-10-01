use crate::url_builder::build_project_url;
use anyhow::Result;
use reqwest::Client;
use smbcloud_model::{
    error_codes::ErrorResponse,
    project::{Project, ProjectCreate},
};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient};

/// `tenant_id` selects which tenant the project is created under, via the
/// `X-Smbcloud-Tenant-Id` header — the API falls back to the user's personal
/// tenant when it's absent, so this is the only way to create a project under
/// an organization tenant.
pub async fn create_project(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    project: ProjectCreate,
    tenant_id: Option<String>,
) -> Result<Project, ErrorResponse> {
    let mut builder = Client::new()
        .post(build_project_url(env, client))
        .json(&project)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    if let Some(tenant_id) = tenant_id {
        builder = builder.header("X-Smbcloud-Tenant-Id", tenant_id);
    }
    request(builder).await
}
