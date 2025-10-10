use {
    crate::config::Config,
    smbcloud_model::error_codes::{ErrorCode, ErrorResponse},
    std::{fs, path::Path},
};

pub fn write_config(repo_path: &str, config: Config) -> Result<(), ErrorResponse> {
    // Ensure .smb directory exists
    let smb_dir = Path::new(&repo_path).join(".smb");
    if !smb_dir.exists() {
        fs::create_dir(smb_dir).map_err(|_| ErrorResponse::Error {
            error_code: ErrorCode::MissingConfig,
            message: ErrorCode::MissingConfig.message(None).to_string(),
        })?;
    }

    // Write config to .smb/config.toml
    let config_toml = toml::to_string(&config).map_err(|_| ErrorResponse::Error {
        error_code: ErrorCode::MissingConfig,
        message: ErrorCode::MissingConfig.message(None).to_string(),
    })?;

    fs::write(".smb/config.toml", config_toml).map_err(|_| ErrorResponse::Error {
        error_code: ErrorCode::MissingConfig,
        message: ErrorCode::MissingConfig.message(None).to_string(),
    })?;

    Ok(())
}
