use network::environment::Environment;
use smbcloud_networking::{constants::PATH_USERS_SIGN_IN, smb_base_url_builder};

pub(crate) fn build_smb_login_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_SIGN_IN);
    url_builder.build()
}
