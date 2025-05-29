use crate::deploy::config::{Config, ConfigError};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use smbcloud_model::project::{Project, ProjectCreate};
use smbcloud_networking::environment::Environment;
use smbcloud_networking_project::{create_project, get_all};
use std::{env, fs, path::Path};

pub async fn setup(env: Environment) -> Result<Config, ConfigError> {
    let path = env::current_dir().ok();
    let path_str = path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Setup project in {}? y/n", path_str))
        .interact()
        .map_err(|_| ConfigError::MissingConfig)?;

    if !confirm {
        return Err(ConfigError::MissingConfig);
    }

    let projects = match get_all(env).await {
        Ok(x) => x,
        Err(_) => return Err(ConfigError::MissingConfig),
    };

    let project = if !projects.is_empty() {
        select_project(env, projects).await?
    } else {
        create_new_project(env).await?
    };

    let name = project.name.clone();
    let description = project.description.clone();

    // Create config struct
    let config = Config {
        project,
        name,
        description,
    };

    // Ensure .smb directory exists
    let smb_dir = Path::new(".smb");
    if !smb_dir.exists() {
        fs::create_dir(smb_dir).map_err(|_| ConfigError::MissingConfig)?;
    }

    // Write config to .smb/config.toml
    let config_toml = toml::to_string_pretty(&config).map_err(|_| ConfigError::MissingConfig)?;
    fs::write(".smb/config.toml", config_toml).map_err(|_| ConfigError::MissingConfig)?;

    println!("Config saved to .smb/config.toml");

    Ok(config)
}

async fn select_project(env: Environment, projects: Vec<Project>) -> Result<Project, ConfigError> {
    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Use existing project? y/n")
        .interact()
        .map_err(|_| ConfigError::MissingConfig)?;

    if !confirm {
        return create_new_project(env).await;
    }
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&projects)
        .default(0)
        .interact()
        .map_err(|_| ConfigError::MissingConfig)?;

    let project = projects[selection].clone();

    Ok(project)
}

async fn create_new_project(env: Environment) -> Result<Project, ConfigError> {
    let name = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            return Err(ConfigError::MissingConfig);
        }
    };
    let repository = match Input::<String>::with_theme(&ColorfulTheme::default())
        .default(name.clone())
        .with_prompt("Repository")
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            return Err(ConfigError::MissingConfig);
        }
    };
    let description = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Description")
        .interact()
    {
        Ok(description) => description,
        Err(_) => {
            return Err(ConfigError::MissingConfig);
        }
    };

    match create_project(
        env,
        ProjectCreate {
            name,
            repository,
            description,
        },
    )
    .await
    {
        Ok(project) => Ok(project),
        Err(_) => Err(ConfigError::MissingConfig),
    }
}
