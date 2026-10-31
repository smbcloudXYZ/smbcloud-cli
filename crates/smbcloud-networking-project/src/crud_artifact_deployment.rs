use {
    crate::url_builder::build_artifact_deployments_url,
    reqwest::Client,
    smbcloud_model::{artifact_deployment::ArtifactDeployment, error_codes::ErrorResponse},
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient},
};

/// `GET /v1/apps/:app_id/artifact_deployments`, newest first.
pub async fn list_artifact_deployments(
    environment: Environment,
    client: (&SmbClient, &str),
    access_token: &str,
    app_id: &str,
) -> Result<Vec<ArtifactDeployment>, ErrorResponse> {
    let builder = Client::new()
        .get(build_artifact_deployments_url(
            environment,
            client,
            app_id,
            None,
        ))
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

/// `POST /v1/apps/:app_id/artifact_deployments/:id/rollback`: republishes a
/// release the server still keeps.
pub async fn rollback_artifact_deployment(
    environment: Environment,
    client: (&SmbClient, &str),
    access_token: &str,
    app_id: &str,
    deployment_id: &str,
) -> Result<ArtifactDeployment, ErrorResponse> {
    let builder = Client::new()
        .post(build_artifact_deployments_url(
            environment,
            client,
            app_id,
            Some(&format!("{deployment_id}/rollback")),
        ))
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
