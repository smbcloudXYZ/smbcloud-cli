use {
    crate::token::smb_token_file_path::smb_token_file_path, anyhow::Result,
    smbcloud_network::environment::Environment, std::fs,
};

pub fn clear_smb_token(env: Environment) -> Result<()> {
    if let Some(path) = smb_token_file_path(env) {
        fs::remove_file(path)?;
    }
    Ok(())
}
