use {
    smbcloud_network::environment::Environment,
    smbcloud_networking::{smb_base_url_builder, smb_client::SmbClient},
};

pub(crate) fn build_auth_apps_url(
    env: Environment,
    client: (&SmbClient, &str),
    project_id: Option<&str>,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/auth_apps");
    if let Some(project_id) = project_id {
        url_builder.add_param("project_id", project_id);
    }
    url_builder.build()
}

pub(crate) fn build_auth_app_url(
    env: Environment,
    client: (&SmbClient, &str),
    auth_app_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/auth_apps");
    url_builder.add_route(auth_app_id);
    url_builder.build()
}
