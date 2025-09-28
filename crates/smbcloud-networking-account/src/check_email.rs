use {
    reqwest::Client,
    smbcloud_model::{account::SmbAuthorization, error_codes::ErrorResponse},
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::PATH_USERS_CHECK_EMAIL, smb_base_url_builder},
};

pub async fn check_email(env: Environment, email: &str) -> Result<SmbAuthorization, ErrorResponse> {
    let builder = Client::new()
        .get(build_url(env, email))
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");
    request(builder).await
}

fn build_url(env: Environment, email: &str) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_CHECK_EMAIL);
    url_builder.add_param("email", email);
    url_builder.build()
}
