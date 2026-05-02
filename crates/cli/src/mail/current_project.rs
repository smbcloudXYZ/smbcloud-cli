use anyhow::{anyhow, Result};
use smbcloud_model::project::Config;
use smbcloud_network::environment::Environment;

pub(crate) fn resolve_optional_project_id(
    env: Environment,
    project_id: Option<String>,
) -> Result<Option<String>> {
    match project_id {
        Some(project_id) => Ok(Some(project_id)),
        None => current_project_id(env),
    }
}

pub(crate) fn resolve_required_project_id(
    env: Environment,
    project_id: Option<String>,
) -> Result<String> {
    match resolve_optional_project_id(env, project_id)? {
        Some(project_id) => Ok(project_id),
        None => Err(anyhow!(
            "No project selected. Pass `--project-id` or run `smb project use --id <project-id>`."
        )),
    }
}

fn current_project_id(env: Environment) -> Result<Option<String>> {
    let home_directory =
        home::home_dir().ok_or_else(|| anyhow!("Failed to resolve your home directory."))?;
    let config_path = home_directory.join(env.smb_dir()).join("config.json");

    if !config_path.exists() {
        return Ok(None);
    }

    let config_content = std::fs::read_to_string(&config_path)
        .map_err(|error| anyhow!("Failed to read `{}`: {}", config_path.display(), error))?;

    let config: Config = serde_json::from_str(&config_content)
        .map_err(|error| anyhow!("Failed to parse `{}`: {}", config_path.display(), error))?;

    Ok(config.current_project.map(|project| project.id.to_string()))
}
