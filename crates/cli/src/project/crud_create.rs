use crate::token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in};
use crate::{
    account::login::process_login,
    cli::CommandResult,
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use console::style;
use dialoguer::console::Term;
use dialoguer::Select;
use dialoguer::{theme::ColorfulTheme, Input};
use smbcloud_model::project::ProjectCreate;
use smbcloud_model::runner::Runner;
use smbcloud_network::environment::Environment;
use smbcloud_networking_project::crud_project_create::create_project;
use spinners::Spinner;

pub async fn process_project_init(
    env: Environment,
    should_init_project: bool,
) -> Result<CommandResult> {
    let is_logged_in = is_logged_in(env).await?;
    if !is_logged_in {
        let _ = process_login(env, Some(is_logged_in)).await;
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

    let runners = vec![Runner::NodeJs, Runner::Swift, Runner::Ruby];
    let runner = Select::with_theme(&ColorfulTheme::default())
        .items(&runners)
        .default(0)
        .interact_on_opt(&Term::stderr())
        .map(|i| runners[i.unwrap()])
        .unwrap();

    let repository = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Repository name")
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            return Err(anyhow!(fail_message("Invalid repository name.")));
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

    if should_init_project {
        setup_smb_folder(&project_name, &description).await?;
    }

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Creating a project...").green().bold().to_string(),
    );

    let access_token = get_smb_token(env)?;
    match create_project(
        env,
        access_token,
        ProjectCreate {
            name: project_name.clone(),
            runner,
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
    let now = Utc::now().to_rfc3339();
    std::fs::write(
        ".smb/config.toml",
        format!(
            r#"
name = "{name}"
description = "{description}"
[project]
id = 1
name = "{repository_name}"
repository = "{repository_name}"
description = "{description}"
created_at = "{now}"
updated_at = "{now}"
"#,
        ),
    )?;
    Ok(())
}
