use {
    crate::url_builder::build_frontend_apps_url,
    reqwest::Client,
    smbcloud_model::{error_codes::ErrorResponse, frontend_app::FrontendApp},
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient},
};

pub async fn get_frontend_apps_by_project(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    project_id: i32,
) -> Result<Vec<FrontendApp>, ErrorResponse> {
    let url = format!(
        "{}?project_id={}",
        build_frontend_apps_url(env, client),
        project_id
    );
    let builder = Client::new()
        .get(url)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
