use {
    smbcloud_network::environment::Environment,
    smbcloud_networking::{smb_base_url_builder, smb_client::SmbClient},
};

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
