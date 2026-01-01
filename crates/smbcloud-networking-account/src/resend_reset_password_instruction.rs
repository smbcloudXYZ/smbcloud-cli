use {
    reqwest::Client,
    smbcloud_model::{account::SmbAuthorization, error_codes::ErrorResponse},
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{
        constants::PATH_RESET_PASSWORD_INSTRUCTIONS, smb_base_url_builder, smb_client::SmbClient,
    },
};

pub async fn resend_reset_password_instruction(
    env: Environment,
    client: SmbClient,
    email: String,
) -> Result<SmbAuthorization, ErrorResponse> {
    let builder = Client::new()
        .post(build_smb_resend_reset_password_instructions_url(
            env, &client,
        ))
        .body(format!("email={}", email))
        .header("User-agent", client.id())
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");

    request(builder).await
}

fn build_smb_resend_reset_password_instructions_url(
    env: Environment,
    client: &SmbClient,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_RESET_PASSWORD_INSTRUCTIONS);
    url_builder.build()
}
