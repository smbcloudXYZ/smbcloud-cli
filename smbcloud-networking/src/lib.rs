pub mod constants;

use crate::constants::TOKEN_PATH_STR;
use anyhow::{anyhow, Result};
use constants::{SMB_API_HOST, SMB_API_PROTOCOL, SMB_CLIENT_ID, SMB_CLIENT_SECRET};
use log::debug;
use std::path::PathBuf;
use url_builder::URLBuilder;

pub async fn get_smb_token() -> Result<String> {
    if let Some(path) = smb_token_file_path() {
        std::fs::read_to_string(path).map_err(|e| {
            debug!("Error while reading token: {}", &e);
            anyhow!("Error while reading token. Are you logged in?")
        })
    } else {
        Err(anyhow!("Failed to get home directory."))
    }
}

pub fn smb_token_file_path() -> Option<PathBuf> {
    match home::home_dir() {
        Some(path) => {
            debug!("Home directory: {}.", path.to_str().unwrap());
            let token_path = path.join(TOKEN_PATH_STR);
            if token_path.exists() && token_path.is_file() {
                return Some(token_path);
            }
            None
        }
        None => {
            debug!("Failed to get home directory.");
            None
        }
    }
}

pub fn smb_base_url_builder() -> URLBuilder {
    let mut url_builder = URLBuilder::new();
    url_builder
        .set_protocol(SMB_API_PROTOCOL)
        .set_host(SMB_API_HOST)
        .add_param("client_id", SMB_CLIENT_ID)
        .add_param("client_secret", SMB_CLIENT_SECRET);
    url_builder
}
