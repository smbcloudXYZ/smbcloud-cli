use {
    reqwest::Client,
    smbcloud_model::{account::SmbAuthorization, error_codes::ErrorResponse},
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::PATH_RESET_PASSWORD_INSTRUCTIONS, smb_client::SmbClient},
    url_builder::URLBuilder,
};

pub async fn resend_reset_password_instruction(
    env: Environment,
    client: (&SmbClient, &str),
    email: String,
) -> Result<SmbAuthorization, ErrorResponse> {
    let builder = Client::new()
        .post(build_smb_resend_reset_password_instructions_url(
            env, client,
        ))
        .body(format!("email={}", email))
        .header("User-agent", client.0.id())
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");

    request(builder).await
}

pub fn build_resend_reset_password_instructions_url(
    env: Environment,
    client_id: &str,
    client_secret: &str,
) -> String {
    let mut url_builder = URLBuilder::new();
    url_builder
        .set_protocol(&env.api_protocol())
        .set_host(&env.api_host())
        .add_param("client_id", client_id)
        .add_param("client_secret", client_secret)
        .add_route(PATH_RESET_PASSWORD_INSTRUCTIONS);
    url_builder.build()
}

fn build_smb_resend_reset_password_instructions_url(
    env: Environment,
    client: (&SmbClient, &str),
) -> String {
    build_resend_reset_password_instructions_url(env, client.0.id(), client.1)
}
