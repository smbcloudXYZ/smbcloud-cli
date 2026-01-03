pub mod constants;
pub mod smb_client;

use {
    crate::smb_client::SmbClient, smbcloud_network::environment::Environment,
    url_builder::URLBuilder,
};

pub fn smb_base_url_builder(env: Environment, client: (&SmbClient, &str)) -> URLBuilder {
    let mut url_builder = URLBuilder::new();
    url_builder
        .set_protocol(&env.api_protocol())
        .set_host(&env.api_host())
        .add_param("client_id", client.0.id())
        .add_param("client_secret", client.1);
    url_builder
}
