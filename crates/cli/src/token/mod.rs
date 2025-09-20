use {
    anyhow::{anyhow, Result},
    dirs::home_dir,
    log::debug,
    smbcloud_network::environment::Environment,
    std::path::{Path, PathBuf},
};

pub(crate) mod test_token_validity;

pub async fn get_smb_token(env: Environment) -> Result<String> {
    if let Some(path) = smb_token_file_path(env) {
        std::fs::read_to_string(path).map_err(|e| {
            debug!("Error while reading token: {}", &e);
            anyhow!("Error while reading token. Are you logged in?")
        })
    } else {
        Err(anyhow!("Failed to get home directory. Are you logged in?"))
    }
}

pub fn smb_token_file_path(env: Environment) -> Option<PathBuf> {
    match home_dir() {
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

pub fn remove_smb_token(env: Environment) -> Result<()> {
    if let Some(path) = smb_token_file_path(env) {
        std::fs::remove_file(path).map_err(|e| {
            debug!("Error while removing token: {}", &e);
            anyhow!("Error while removing token. Are you logged in?")
        })
    } else {
        Err(anyhow!("Failed to get home directory. Are you logged in?"))
    }
}
