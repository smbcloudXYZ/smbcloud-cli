use {
    smbcloud_network::environment::Environment,
    smbcloud_networking::{smb_base_url_builder, smb_client::SmbClient},
};

pub(crate) fn build_deploy_repos_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/deploy_repos");
    url_builder.build()
}

pub(crate) fn build_github_installations_url(
    env: Environment,
    client: (&SmbClient, &str),
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/github/installations");
    url_builder.build()
}

pub(crate) fn build_github_installation_repositories_url(
    env: Environment,
    client: (&SmbClient, &str),
    installation_id: i64,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/github/installations");
    url_builder.add_route(&installation_id.to_string());
    url_builder.add_route("repositories");
    url_builder.build()
}

pub(crate) fn build_deploy_repo_github_connection_url(
    env: Environment,
    client: (&SmbClient, &str),
    deploy_repo_id: i64,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/deploy_repos");
    url_builder.add_route(&deploy_repo_id.to_string());
    url_builder.add_route("github_connection");
    url_builder.build()
}

pub(crate) fn build_frontend_apps_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/frontend_apps");
    url_builder.build()
}

pub(crate) fn build_frontend_app_update_deploy_config_url(
    env: Environment,
    client: (&SmbClient, &str),
    frontend_app_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/frontend_apps");
    url_builder.add_route(frontend_app_id);
    url_builder.build()
}

pub(crate) fn build_frontend_app_deploy_config_url(
    env: Environment,
    client: (&SmbClient, &str),
    frontend_app_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/frontend_apps");
    url_builder.add_route(frontend_app_id);
    url_builder.add_route("deploy_config");
    url_builder.build()
}

// Private functions

pub(crate) fn build_project_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/projects");
    url_builder.build()
}

pub(crate) fn build_project_url_with_id(
    env: Environment,
    client: (&SmbClient, &str),
    id: String,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/projects");
    url_builder.add_route(id.as_str());
    url_builder.build()
}

pub(crate) fn build_project_deployment_index(
    env: Environment,
    client: (&SmbClient, &str),
    project_id: String,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/projects");
    url_builder.add_route(project_id.as_str());
    url_builder.add_route("deployment");
    url_builder.build()
}

pub(crate) fn build_project_deployment(
    env: Environment,
    client: (&SmbClient, &str),
    project_id: String,
    id: String,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/projects");
    url_builder.add_route(project_id.as_str());
    url_builder.add_route("deployment");
    url_builder.add_route(id.as_str());
    url_builder.build()
}
