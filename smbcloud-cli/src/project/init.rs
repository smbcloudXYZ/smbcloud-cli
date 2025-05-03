use crate::{account::lib::protected_request, cli::CommandResult, ui::{succeed_message, succeed_symbol}};
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use smbcloud_model::project::ProjectCreate;
use smbcloud_networking::environment::Environment;
use smbcloud_networking_project::create_project;
use spinners::Spinner;

pub async fn process_project_init(env: Environment) -> Result<CommandResult> {
    protected_request(env).await?;

    let project_name = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            let error = anyhow!("Invalid project name.");
            return Err(error);
        }
    };
    let description = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Description")
        .interact()
    {
        Ok(description) => description,
        Err(_) => {
            let error = anyhow!("Invalid description.");
            return Err(error);
        }
    };

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Creating a project...").green().bold().to_string(),
    );

    match create_project(
        env,
        ProjectCreate {
            name: project_name.clone(),
            description: description.clone(),
        },
    )
    .await
    {
        Ok(_) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message(&format!("{project_name} has been created.")),
        }),
        Err(e) => {
            println!("Error: {e:#?}");
            Ok(CommandResult {
                spinner,
                symbol: succeed_symbol(),
                msg: format!("Failed to create a project {project_name}."),
            })
        }
    }
}
