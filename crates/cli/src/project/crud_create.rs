use crate::{
    account::{lib::is_logged_in, login::process_login},
    cli::CommandResult,
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use smbcloud_model::project::ProjectCreate;
use smbcloud_networking::{environment::Environment, get_smb_token};
use smbcloud_networking_project::crud_project_create::create_project;
use spinners::Spinner;

pub async fn process_project_init(env: Environment) -> Result<CommandResult> {
    if !is_logged_in(env) {
        let _ = process_login(env).await;
    }

    let project_name = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            return Err(anyhow!(fail_message("Invalid project name.")));
        }
    };
    let repository = match Input::<String>::with_theme(&ColorfulTheme::default())
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

    let access_token = get_smb_token(env).await?;
    match create_project(
        env,
        access_token,
        ProjectCreate {
            name: project_name.clone(),
            repository,
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
id = 1
name = "{repository_name}"
"#,
        ),
    )?;
    Ok(())
}
