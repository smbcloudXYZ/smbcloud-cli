pub mod constants;

use constants::{SMB_CLIENT_ID, SMB_CLIENT_SECRET};
use network::environment::Environment;
use url_builder::URLBuilder;

pub fn smb_base_url_builder(env: Environment) -> URLBuilder {
    let mut url_builder = URLBuilder::new();
    url_builder
        .set_protocol(&env.api_protocol())
        .set_host(&env.api_host())
        .add_param("client_id", SMB_CLIENT_ID)
        .add_param("client_secret", SMB_CLIENT_SECRET);
    url_builder
}
