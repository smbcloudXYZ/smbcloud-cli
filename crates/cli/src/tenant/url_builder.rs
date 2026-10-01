use {
    smbcloud_network::environment::Environment,
    smbcloud_networking::{smb_base_url_builder, smb_client::SmbClient},
};

pub(crate) fn build_tenants_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/tenants");
    url_builder.build()
}

pub(crate) fn build_tenant_url(
    env: Environment,
    client: (&SmbClient, &str),
    tenant_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/tenants");
    url_builder.add_route(tenant_id);
    url_builder.build()
}
