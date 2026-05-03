use smbcloud_network::environment::Environment;

/// API credentials for a GresIQ-managed database.
///
/// Obtain these from the smbCloud console after registering a GresIQ app.
/// The `api_key` identifies the app; the `api_secret` authenticates it.
#[derive(Clone, Copy)]
pub struct GresiqCredentials<'a> {
    pub api_key: &'a str,
    pub api_secret: &'a str,
}

/// Resolve the GresIQ gateway base URL for the given environment.
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
