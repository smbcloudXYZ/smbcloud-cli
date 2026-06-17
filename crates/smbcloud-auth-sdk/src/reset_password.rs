use {
    crate::client_credentials::{ClientCredentials, base_url_builder as tenant_base_url_builder},
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        forgot::{Args, Email},
        reset_password_response::ResetPasswordResponse,
    },
    smbcloud_network::{environment::Environment, network::request},
};

/// Requests password-reset instructions for a tenant `AuthUser`.
///
/// Hits `POST /v1/client/users/reset_password`. The endpoint never reveals
/// whether the email belongs to a real account (no enumeration), so it always
/// resolves with a neutral message on success. Calling it again re-issues the
/// reset token, so the same call also serves the "resend reset instructions"
/// flow — the tenant plane has no separate resend endpoint.
pub async fn reset_password_with_client(
    env: Environment,
    client: ClientCredentials<'_>,
    email: String,
) -> Result<ResetPasswordResponse, ErrorResponse> {
    let params = Args {
        user: Email { email },
    };
    let builder = Client::new()
        .post(build_reset_password_url(env, client))
        .json(&params)
        .header("User-agent", client.app_id);
    request(builder).await
}

fn build_reset_password_url(env: Environment, client: ClientCredentials<'_>) -> String {
    let mut url_builder = tenant_base_url_builder(env, client);
    url_builder.add_route("v1/client/users/reset_password");
    url_builder.build()
}
