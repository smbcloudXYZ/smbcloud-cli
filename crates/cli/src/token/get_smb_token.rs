use {
    crate::token::smb_token_file_path::smb_token_file_path,
    anyhow::{anyhow, Result},
    log::debug,
    smbcloud_network::environment::Environment,
};

pub fn get_smb_token(env: Environment) -> Result<String> {
    if let Some(path) = smb_token_file_path(env) {
        std::fs::read_to_string(path).map_err(|e| {
            debug!("Error while reading token: {}", &e);
            anyhow!("Error while reading token. Are you logged in?")
        })
    } else {
        Err(anyhow!("Failed to get home directory. Are you logged in?"))
    }
}
