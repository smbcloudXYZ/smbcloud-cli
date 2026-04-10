use {smbcloud_network::environment::Environment, url_builder::URLBuilder};

#[derive(Clone, Copy)]
pub struct ClientCredentials<'a> {
    pub app_id: &'a str,
    pub app_secret: &'a str,
}

pub fn base_url_builder(env: Environment, client: ClientCredentials<'_>) -> URLBuilder {
    let mut url_builder = URLBuilder::new();
    url_builder
        .set_protocol(&env.api_protocol())
        .set_host(&env.api_host())
        .add_param("client_id", client.app_id)
        .add_param("client_secret", client.app_secret);
    url_builder
}
