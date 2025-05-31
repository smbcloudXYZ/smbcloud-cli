use crate::{
    deploy::config::{Config, ConfigError},
    ui::highlight,
};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use regex::Regex;
use smbcloud_model::project::{Project, ProjectCreate};
use smbcloud_networking::{environment::Environment, get_smb_token};
use smbcloud_networking_project::{create_project, crud_project_read::get_projects};
use std::{env, fs, path::Path};

pub async fn setup_project(env: Environment) -> Result<Config, ConfigError> {
    let path = env::current_dir().ok();
    let path_str = path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Setup project in {}? y/n", highlight(&path_str)))
        .interact()
        .map_err(|_| ConfigError::InputError)?;

    if !confirm {
        return Err(ConfigError::Cancel);
    }

    let access_token = match get_smb_token(env).await {
        Ok(token) => token,
        Err(_) => return Err(ConfigError::MissingToken),
    };

    let projects = match get_projects(env, access_token).await {
        Ok(x) => x,
        Err(_) => return Err(ConfigError::InputError),
    };

    let project: Project = if !projects.is_empty() {
        select_project(env, projects, &path_str).await?
    } else {
        create_new_project(env, &path_str).await?
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
    let config_toml = toml::to_string(&config).map_err(|_| ConfigError::MissingConfig)?;
    fs::write(".smb/config.toml", config_toml).map_err(|_| ConfigError::MissingConfig)?;

    Ok(config)
}

async fn select_project(
    env: Environment,
    projects: Vec<Project>,
    path: &str,
) -> Result<Project, ConfigError> {
    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Use existing project? y/n")
        .interact()
        .map_err(|_| ConfigError::InputError)?;

    if !confirm {
        return create_new_project(env, path).await;
    }
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&projects)
        .default(0)
        .interact()
        .map_err(|_| ConfigError::InputError)?;

    let project = projects[selection].clone();

    Ok(project)
}

async fn create_new_project(env: Environment, path: &str) -> Result<Project, ConfigError> {
    let default_name = Path::new(path)
        .file_name()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("project");

    let name = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .default(default_name.to_string())
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            return Err(ConfigError::InputError);
        }
    };

    // Create a repository name: lowercased, remove spaces and special characters
    let re = Regex::new(r"[^a-zA-Z0-9_-]").unwrap();
    let default_repository = name.clone().to_lowercase().replace(' ', "_");
    let default_repo = re.replace_all(&default_repository, "");

    let repository = match Input::<String>::with_theme(&ColorfulTheme::default())
        .default(default_repo.to_string())
        .with_prompt("Repository")
        .interact()
    {
        Ok(repo) => repo,
        Err(_) => {
            return Err(ConfigError::InputError);
        }
    };

    let description = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Description")
        .interact()
    {
        Ok(description) => description,
        Err(_) => {
            return Err(ConfigError::InputError);
        }
    };

    let access_token = match get_smb_token(env).await {
        Ok(token) => token,
        Err(_) => return Err(ConfigError::MissingToken),
    };

    match create_project(
        env,
        access_token,
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
