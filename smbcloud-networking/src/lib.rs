pub mod constants;
pub mod environment;

use anyhow::{anyhow, Result};
use constants::{SMB_CLIENT_ID, SMB_CLIENT_SECRET};
use environment::Environment;
use log::debug;
use std::path::{Path, PathBuf};
use url_builder::URLBuilder;

pub async fn get_smb_token(env: Environment) -> Result<String> {
    if let Some(path) = smb_token_file_path(env) {
        std::fs::read_to_string(path).map_err(|e| {
            debug!("Error while reading token: {}", &e);
            anyhow!("Error while reading token. Are you logged in?")
        })
    } else {
        Err(anyhow!("Failed to get home directory."))
    }
}

pub fn smb_token_file_path(env: Environment) -> Option<PathBuf> {
    match home::home_dir() {
        Some(home_path) => {
            let token_path = [&env.smb_dir(), "/token"].join("");
            let token_file = home_path.join(Path::new(&token_path));
            if token_file.exists() && token_file.is_file() {
                return Some(token_file);
            }
            None
        }
        None => {
            debug!("Failed to get home directory.");
            None
        }
    }
}

pub fn smb_base_url_builder(env: Environment) -> URLBuilder {
    let mut url_builder = URLBuilder::new();
    url_builder
        .set_protocol(&env.api_protocol())
        .set_host(&env.api_host())
        .add_param("client_id", SMB_CLIENT_ID)
        .add_param("client_secret", SMB_CLIENT_SECRET);
    url_builder
}
