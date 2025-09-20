use {
    dirs::home_dir,
    log::debug,
    smbcloud_network::environment::Environment,
    std::path::{Path, PathBuf},
};

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
