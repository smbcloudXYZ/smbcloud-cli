use crate::{
    cli::CommandResult,
    project::{
        cli::Commands,
        crud_create::process_project_init,
        crud_delete::process_project_delete,
        crud_read::{process_project_list, process_project_show, process_project_use},
        crud_update::process_project_update_description,
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
        Commands::Use { id } => process_project_use(env, id).await,
        Commands::Deployment { id } => process_deployment(env, id).await,
        Commands::Update { id } => process_project_update_description(env, id).await,
    }
}
