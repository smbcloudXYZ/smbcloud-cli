use crate::client;
use crate::token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in};
use crate::{
    account::login::process_login,
    cli::CommandResult,
    project::deploy_target::{
        ensure_default_frontend_app_for_project, merge_project_with_frontend_app,
    },
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::console::Term;
use dialoguer::Select;
use dialoguer::{theme::ColorfulTheme, Input};
use smbcloud_model::frontend_app::FrontendApp;
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

    // Runner and repository describe the project's first app — the deployable
    // unit inside the workspace — not the workspace itself.
    let runners = vec![
        Runner::NodeJs,
        Runner::Static,
        Runner::Ruby,
        Runner::Swift,
        Runner::Rust,
    ];
    let runner = match Select::with_theme(&ColorfulTheme::default())
        .items(&runners)
        .default(0)
        .interact_on_opt(&Term::stderr())
    {
        Ok(Some(index)) => runners[index],
        _ => {
            return Err(anyhow!(fail_message("Invalid runner.")));
        }
    };

    let repository = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Repository name")
        .interact()
    {
        Ok(repository) => repository,
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
    let project = match create_project(
        env,
        client(),
        access_token.clone(),
        ProjectCreate {
            name: project_name.clone(),
            description: description.clone(),
        },
    )
    .await
    {
        Ok(project) => project,
        Err(e) => {
            println!("Error: {e:#?}");
            return Err(anyhow!(fail_message("Failed to create project.")));
        }
    };

    let frontend_app = match ensure_default_frontend_app_for_project(
        env,
        &access_token,
        &project,
        runner,
        Some(repository),
    )
    .await
    {
        Ok(frontend_app) => frontend_app,
        Err(e) => {
            println!("Error: {e:#?}");
            return Err(anyhow!(fail_message(
                "Project created, but failed to create its app."
            )));
        }
    };

    if should_init_project {
        write_smb_config(&project, &frontend_app)?;
    }

    Ok(CommandResult {
        spinner,
        symbol: succeed_symbol(),
        msg: succeed_message(&format!("{project_name} has been created.")),
    })
}

fn write_smb_config(workspace_project: &Project, frontend_app: &FrontendApp) -> Result<()> {
    let deploy_target = merge_project_with_frontend_app(workspace_project, frontend_app);

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
