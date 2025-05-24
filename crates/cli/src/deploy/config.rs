use std::{fs, path::Path};

use crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol};
use anyhow::Result;
use console::style;
use git2::{Cred, CredentialType, Error};
use spinners::Spinner;
use toml::Value;

pub(crate) async fn check_config() -> Result<Config> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Checking config...").green().bold().to_string(),
    );

    // Check .smb directory

    // Get .smb/config.toml file path in the current directory
    let config_path = Path::new(".smb/config.toml");
    if !config_path.exists() {
        spinner.stop_and_persist(&fail_symbol(), fail_message("Invalid config."));
        return Err(anyhow::anyhow!(fail_message(
            "No config file found. Please run `smbcloud init` command."
        )));
    }

    // Parse toml file
    let config_content = fs::read_to_string(config_path)?;
    let config: Value = toml::from_str(&config_content)?;

    let repo_name = config
        .get("repository")
        .and_then(|repo| repo.get("name"))
        .and_then(|name| name.as_str())
        .ok_or_else(|| anyhow::anyhow!(fail_message("Repo name not found in config file.")))?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Valid config."));

    Ok(Config {
        name: repo_name.to_owned(),
    })
}

pub struct Config {
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
