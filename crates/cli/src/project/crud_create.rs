use crate::deploy::config::check_config;
use crate::{
    account::lib::protected_request,
    cli::CommandResult,
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use smbcloud_model::project::ProjectCreate;
use smbcloud_networking::environment::Environment;
use smbcloud_networking_project::create_project;
use spinners::Spinner;

pub async fn process_project_init(env: Environment) -> Result<CommandResult> {
    protected_request(env).await?;

    // Check config.
    let config = check_config().await?;

    let project_name = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            return Err(anyhow!(fail_message("Invalid project name.")));
        }
    };
    let description = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Description")
        .interact()
    {
        Ok(description) => description,
        Err(_) => {
            return Err(anyhow!(fail_message("Invalid description")));
        }
    };

    setup_smb_folder(&project_name, &description).await?;

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
            Err(anyhow!(fail_message("Failed to create project.")))
        }
    }
}

async fn setup_smb_folder(name: &str, description: &str) -> Result<()> {
    // Create .smb folder in the current directory
    std::fs::create_dir(".smb")?;
    // Create config.toml file in the .smb folder
    let repository_name = name.to_lowercase().replace(" ", "");
    std::fs::write(
        ".smb/config.toml",
        format!(
            r#"
name = "{name}"
description = "{description}"
[repository]
name = "{repository_name}"
"#,
        ),
    )?;
    Ok(())
}
