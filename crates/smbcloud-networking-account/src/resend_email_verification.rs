use {
    reqwest::Client,
    smbcloud_model::{account::SmbAuthorization, error_codes::ErrorResponse},
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{
        constants::PATH_RESEND_CONFIRMATION, smb_base_url_builder, smb_client::SmbClient,
    },
};

pub async fn resend_email_verification(
    env: Environment,
    client: (&SmbClient, &str),
    email: String,
) -> Result<SmbAuthorization, ErrorResponse> {
    let builder = Client::new()
        .post(build_smb_resend_email_verification_url(env, client))
        .body(format!("email={}", email))
        .header("User-agent", client.0.id())
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");

    request(builder).await
}

fn build_smb_resend_email_verification_url(env: Environment, client: (&SmbClient, &str)) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_RESEND_CONFIRMATION);
    url_builder.build()
}
