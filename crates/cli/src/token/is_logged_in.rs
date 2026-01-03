use {
    crate::{client, token::get_smb_token::get_smb_token},
    smbcloud_model::error_codes::ErrorResponse,
    smbcloud_network::environment::Environment,
    smbcloud_networking_account::me::me,
    tracing::debug,
};

pub async fn is_logged_in(env: Environment) -> Result<bool, ErrorResponse> {
    // Check if token is valid
    let access_token = match get_smb_token(env) {
        Ok(token) => token,
        Err(_) => return Ok(false),
    };
    match me(env, client(), &access_token).await {
        Ok(user) => {
            debug!("Authorized as: {:?}", user.id);
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}
