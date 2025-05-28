use std::{fs, path::Path};

use crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol};
use git2::{Cred, CredentialType, Error};
use serde::Deserialize;
use spinners::Spinner;
use thiserror::Error;

pub(crate) async fn check_config() -> Result<Config, ConfigError> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Checking config"),
    );

    // Check .smb directory

    // Get .smb/config.toml file path in the current directory
    let config_path = Path::new(".smb/config.toml");
    if !config_path.exists() {
        spinner.stop_and_persist(&fail_symbol(), fail_message("Invalid config."));
        return Err(ConfigError::MissingConfig);
    }

    // Parse toml file
    let config_content = fs::read_to_string(config_path).map_err(|_| ConfigError::MissingConfig)?;

    let config: Config = match toml::from_str(&config_content) {
        Ok(value) => value,
        Err(_) => {
            spinner.stop_and_persist(&fail_symbol(), fail_message("Config unsync."));
            handle_config_error()?
        }
    };

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Valid config."));

    Ok(config)
}

fn handle_config_error() -> Result<Config, ConfigError> {
    todo!()
}

#[derive(Deserialize)]
pub struct Config {
    pub name: String,
    pub description: String,
    pub repository: Repository,
}

#[derive(Deserialize)]
pub struct Repository {
    pub id: i32,
    pub name: String,
}

impl Config {
    pub fn credentials(
        &self,
    ) -> impl FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, Error> + '_ {
        move |_url, _username_from_url, _allowed_types| {
            Cred::ssh_key("git", None, Path::new(&self.ssh_key_path()), None)
        }
    }

    fn ssh_key_path(&self) -> String {
        // Use the dirs crate to get the home directory
        let home = dirs::home_dir().expect("Could not determine home directory");
        let key_path = home
            .join(".ssh")
            .join(format!("id_{}@smbcloud.xyz", self.name));
        let key_path_str = key_path.to_string_lossy().to_string();
        println!("Use key path: {}", key_path_str);
        key_path_str
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConfigError {
    #[error("Missing config.")]
    MissingConfig,
    #[error("Missing id in repository")]
    MissingId,
}
