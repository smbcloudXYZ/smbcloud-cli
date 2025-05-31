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
        crud_read::{process_project_list, process_project_show, process_project_use},
        deployment::process_deployment,
    },
};
use anyhow::Result;
use smbcloud_networking::environment::Environment;

pub async fn process_project(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::New {} => process_project_init(env).await,
        Commands::List {} => process_project_list(env).await,
        Commands::Show { id } => process_project_show(env, id).await,
        Commands::Delete { id } => process_project_delete(env, id).await,
        Commands::Use { id } => { process_project_use(env, id).await }
        Commands::Deployment { id } => process_deployment(env, id).await,
    }
}
