use {
    crate::url_builder::build_app_deploy_config_url,
    reqwest::{Client, StatusCode},
    smbcloud_model::{
        deploy_config::DeployConfig,
        error_codes::{ErrorCode, ErrorResponse},
    },
    smbcloud_network::{environment::Environment, network::parse_error_response},
    smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient},
};

/// `GET /v1/apps/:app_id/deploy_config`. Unlike the `frontend_apps` route, an
/// app's own deploy token may call this, so a CI job can learn the app's
/// `publish_target` without a user session.
///
/// A 401 or 403 comes back as `ErrorCode::Unauthorized` and a 404 as
/// `ErrorCode::ProjectNotFound`, whatever the body says, so the caller can tell
/// a refused token from a missing app.
pub async fn get_app_deploy_config(
    environment: Environment,
    client: (&SmbClient, &str),
    access_token: &str,
    app_id: &str,
) -> Result<DeployConfig, ErrorResponse> {
    let builder = Client::new()
        .get(build_app_deploy_config_url(environment, client, app_id))
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-agent", SMB_USER_AGENT);
    let response = builder.send().await.map_err(|_| ErrorResponse::Error {
        error_code: ErrorCode::NetworkError,
        message: ErrorCode::NetworkError.message(None).to_string(),
    })?;
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ErrorResponse::Error {
            error_code: ErrorCode::Unauthorized,
            message: ErrorCode::Unauthorized.message(None).to_string(),
        }),
        StatusCode::NOT_FOUND => Err(ErrorResponse::Error {
            error_code: ErrorCode::ProjectNotFound,
            message: ErrorCode::ProjectNotFound.message(None).to_string(),
        }),
        status if status.is_success() => response.json().await.map_err(|_| ErrorResponse::Error {
            error_code: ErrorCode::ParseError,
            message: ErrorCode::ParseError.message(None).to_string(),
        }),
        _ => parse_error_response(response).await,
    }
}
