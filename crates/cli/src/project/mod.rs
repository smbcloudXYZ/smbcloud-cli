pub mod cli;
pub mod crud_create;
pub mod crud_delete;
pub mod crud_read;
mod deployment;

use self::cli::Commands;
use crate::{
    cli::CommandResult,
    project::{
        crud_create::process_project_init,
        crud_delete::process_project_delete,
        crud_read::{process_project_list, process_project_show},
        deployment::process_deployment,
    },
    ui::{succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use log::debug;
use smbcloud_model::project::Config;
use smbcloud_networking::environment::Environment;
use smbcloud_networking_project::get_project;
use spinners::Spinner;
use std::{fs::OpenOptions, io::Write};

pub async fn process_project(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::New {} => process_project_init(env).await,
        Commands::List {} => process_project_list(env).await,
        Commands::Show { id } => process_project_show(env, id).await,
        Commands::Delete { id } => process_project_delete(env, id).await,
        Commands::Use { id } => {
            let project = get_project(env,"access token".to_owned(),  id).await?;

            let config = Config {
                current_project: Some(project),
                current_auth_app: None,
            };

            let spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Loading"),
            );
            match home::home_dir() {
                Some(path) => {
                    debug!("{}", path.to_str().unwrap());
                    let mut file = OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .write(true)
                        .open([path.to_str().unwrap(), "/.smb/config.json"].join(""))?;
                    let json = serde_json::to_string(&config)?;
                    file.write_all(json.as_bytes())?;

                    Ok(CommandResult {
                        spinner,
                        symbol: succeed_symbol(),
                        msg: succeed_message("Use project successful."),
                    })
                }
                None => {
                    let error = anyhow!("Failed to get home directory.");
                    Err(error)
                }
            }
        }
        Commands::Deployment { id } => process_deployment(env, id).await,
    }
}
