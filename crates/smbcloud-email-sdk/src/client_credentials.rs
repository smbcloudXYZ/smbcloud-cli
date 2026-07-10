use smbcloud_network::environment::Environment;

/// API credential for the smbCloud transactional email API.
///
/// `api_key` is a `smb_mail_…` key minted for a Mail app in the smbCloud
/// console. It is sent as `Authorization: Bearer <api_key>` and scopes sending
/// to that app's verified domain.
#[derive(Clone, Copy)]
pub struct EmailCredentials<'a> {
    pub api_key: &'a str,
}

/// Resolve the email API base URL for the given environment.
///
/// - **Dev** → `http://localhost:8088`
/// - **Production** → `https://api.smbcloud.xyz`
pub fn base_url(environment: &Environment) -> String {
    format!(
        "{}://{}",
        environment.api_protocol(),
        environment.api_host()
    )
}
