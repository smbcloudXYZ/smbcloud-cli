use anyhow::{anyhow, Result};
use smbcloud_network::environment::Environment;

pub(crate) fn resolve_optional_project_id(
    env: Environment,
    project_id: Option<String>,
) -> Result<Option<String>> {
    match project_id {
        Some(project_id) => Ok(Some(project_id)),
        None => crate::session_config::current_project_id(env),
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
