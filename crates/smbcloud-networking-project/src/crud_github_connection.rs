use {
    crate::url_builder::{
        build_deploy_repo_github_connection_url, build_github_installation_repositories_url,
        build_github_installations_url,
    },
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        github::{
            GithubConnection, GithubConnectionCreate, GithubConnectionStatus, GithubInstallation,
            GithubRepository,
        },
    },
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient},
};

pub async fn get_github_installations(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
) -> Result<Vec<GithubInstallation>, ErrorResponse> {
    let builder = Client::new()
        .get(build_github_installations_url(env, client))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_github_installation_repositories(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    installation_id: i64,
) -> Result<Vec<GithubRepository>, ErrorResponse> {
    let builder = Client::new()
        .get(build_github_installation_repositories_url(
            env,
            client,
            installation_id,
        ))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_github_connection(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    deploy_repo_id: i64,
) -> Result<GithubConnectionStatus, ErrorResponse> {
    let builder = Client::new()
        .get(build_deploy_repo_github_connection_url(
            env,
            client,
            deploy_repo_id,
        ))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn create_github_connection(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    deploy_repo_id: i64,
    payload: GithubConnectionCreate,
) -> Result<GithubConnection, ErrorResponse> {
    let builder = Client::new()
        .post(build_deploy_repo_github_connection_url(
            env,
            client,
            deploy_repo_id,
        ))
        .json(&payload)
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn delete_github_connection(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    deploy_repo_id: i64,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_deploy_repo_github_connection_url(
            env,
            client,
            deploy_repo_id,
        ))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}
