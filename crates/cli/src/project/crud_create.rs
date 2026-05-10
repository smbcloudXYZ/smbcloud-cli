use crate::client;
use crate::token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in};
use crate::{
    account::login::process_login,
    cli::CommandResult,
    project::deploy_target::{
        ensure_default_frontend_app_for_project, merge_project_with_frontend_app,
        resolve_frontend_app_for_project,
    },
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::console::Term;
use dialoguer::Select;
use dialoguer::{theme::ColorfulTheme, Input};
use smbcloud_model::project::{Project, ProjectCreate};
use smbcloud_model::runner::Runner;
use smbcloud_network::environment::Environment;
use smbcloud_networking_project::crud_project_create::create_project;
use smbcloud_utils::config::Config as DeployConfig;
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

    let runners = vec![
        Runner::NodeJs,
        Runner::Static,
        Runner::Ruby,
        Runner::Swift,
        Runner::Rust,
    ];
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

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Creating a project...").green().bold().to_string(),
    );

    let access_token = get_smb_token(env)?;
    match create_project(
        env,
        client(),
        access_token.clone(),
        ProjectCreate {
            name: project_name.clone(),
            runner,
            repository,
            description: description.clone(),
            deployment_method: Default::default(),
        },
    )
    .await
    {
        Ok(project) => {
            if should_init_project {
                let _ = ensure_default_frontend_app_for_project(env, &access_token, &project).await;
                write_smb_config(env, &access_token, project.clone()).await?;
            }

            Ok(CommandResult {
                spinner,
                symbol: succeed_symbol(),
                msg: succeed_message(&format!("{project_name} has been created.")),
            })
        }
        Err(e) => {
            println!("Error: {e:#?}");
            Err(anyhow!(fail_message("Failed to create project.")))
        }
    }
}

async fn write_smb_config(
    env: Environment,
    access_token: &str,
    workspace_project: Project,
) -> Result<()> {
    let deploy_target = match resolve_frontend_app_for_project(
        env,
        access_token,
        &workspace_project,
        false,
    )
    .await
    {
        Ok(Some(frontend_app)) => {
            merge_project_with_frontend_app(&workspace_project, &frontend_app)
        }
        _ => workspace_project.clone(),
    };

    let config = DeployConfig {
        name: workspace_project.name.clone(),
        description: workspace_project.description.clone(),
        project: deploy_target,
        projects: None,
    };

    std::fs::create_dir_all(".smb")?;
    std::fs::write(".smb/config.toml", toml::to_string(&config)?)?;
    Ok(())
}
