use crate::{
    deploy::setup::setup_project,
    ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
};
use git2::{Cred, CredentialType, Error};
use serde::{Deserialize, Serialize};
use smbcloud_model::{account::User, project::Project};
use smbcloud_networking::environment::Environment;
use smbcloud_networking_project::crud_project_read::get_project;
use spinners::Spinner;
use std::{fs, path::Path};
use thiserror::Error;

pub(crate) async fn check_config(env: Environment) -> Result<Config, ConfigError> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Checking config"),
    );

    // Check .smb directory

    // Get .smb/config.toml file path in the current directory
    let config_path = Path::new(".smb/config.toml");
    if !config_path.exists() {
        spinner.stop_and_persist(&succeed_symbol(), succeed_message("Setting up deployment"));
        setup_project(env).await?;
        spinner = Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            succeed_message("Checking config"),
        );
    }

    // Parse toml file
    let config_content = fs::read_to_string(config_path).map_err(|_| ConfigError::MissingConfig)?;

    let config: Config = match toml::from_str(&config_content) {
        Ok(value) => value,
        Err(e) => {
            println!("{}", e);
            spinner.stop_and_persist(&fail_symbol(), fail_message("Config unsync."));
            handle_config_error()?
        }
    };
    spinner.stop_and_persist(
        &succeed_symbol(),
        succeed_message(&format!("Valid config for {}", config.name)),
    );

    Ok(config)
}

fn handle_config_error() -> Result<Config, ConfigError> {
    todo!()
}

pub(crate) async fn check_project(
    env: Environment,
    access_token: &str,
    id: i32,
) -> Result<(), ConfigError> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::Hamburger,
        succeed_message("Validate project"),
    );
    match get_project(env, access_token.to_string(), id.to_string()).await {
        Ok(_) => {
            spinner.stop_and_persist(&succeed_symbol(), succeed_message("Valid project"));
            Ok(())
        }
        Err(_) => {
            spinner.stop_and_persist(&fail_symbol(), succeed_message("Project is unsynched"));
            Err(ConfigError::ProjectNotFound)
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub name: String,
    pub description: Option<String>,
    pub project: Project,
}

impl Config {
    pub fn credentials(
        &self,
        user: User,
    ) -> impl FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, Error> + '_ {
        move |_url, _username_from_url, _allowed_types| {
            Cred::ssh_key("git", None, Path::new(&self.ssh_key_path(user.id)), None)
        }
    }

    fn ssh_key_path(&self, user_id: i32) -> String {
        // Use the dirs crate to get the home directory
        let home = dirs::home_dir().expect("Could not determine home directory");
        let key_path = home.join(".ssh").join(format!("id_{}@smbcloud", user_id));
        let key_path_str = key_path.to_string_lossy().to_string();
        println!("Use key path: {}", key_path_str);
        key_path_str
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConfigError {
    #[error("Missing config file. Please regenerate with 'smb init'.")]
    MissingConfig,
    #[error("Missing id in repository. Please regenerate with 'smb init'.")]
    MissingId,
    #[error("Could not find project in your list. Make sure you have access to the project.")]
    ProjectNotFound,
    #[error("Cancel operation.")]
    Cancel,
    #[error("Input error.")]
    InputError,
}
