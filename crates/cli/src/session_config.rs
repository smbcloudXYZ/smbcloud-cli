//! Session-level CLI state: the tenant and project selected via `smb tenant
//! use` / `smb project use`, persisted to `~/.smb[-dev]/config.toml`.
//!
//! This is distinct from the project-local `.smb/config.toml` written by
//! `smb project new` / `smb init` (deploy target config for the current
//! working directory, handled by `smbcloud_utils::write_config`) — this file
//! lives in the home directory and tracks which tenant/project the CLI
//! session is currently operating against, independent of cwd.

use anyhow::{anyhow, Result};
use smbcloud_model::{
    frontend_app::FrontendApp, project::Config, project::Project, tenant::Tenant,
};
use smbcloud_network::environment::Environment;
use std::path::PathBuf;

fn config_path(env: Environment) -> Result<PathBuf> {
    let home_directory =
        home::home_dir().ok_or_else(|| anyhow!("Failed to resolve your home directory."))?;
    Ok(home_directory.join(env.smb_dir()).join("config.toml"))
}

pub fn read(env: Environment) -> Result<Config> {
    let path = config_path(env)?;
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|error| anyhow!("Failed to read `{}`: {}", path.display(), error))?;
    toml::from_str(&content)
        .map_err(|error| anyhow!("Failed to parse `{}`: {}", path.display(), error))
}

pub fn write(env: Environment, config: &Config) -> Result<()> {
    let path = config_path(env)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = toml::to_string(config)?;
    std::fs::write(&path, content)
        .map_err(|error| anyhow!("Failed to write `{}`: {}", path.display(), error))?;
    Ok(())
}

pub fn current_project_id(env: Environment) -> Result<Option<String>> {
    Ok(read(env)?
        .current_project
        .map(|project| project.id.to_string()))
}

pub fn current_tenant_id(env: Environment) -> Result<Option<String>> {
    Ok(read(env)?
        .current_tenant
        .map(|tenant| tenant.id.to_string()))
}

/// Selects a project for the session. Clears any previously selected
/// frontend app — it belonged to the old project and no longer applies.
pub fn set_current_project(
    env: Environment,
    project: Project,
    frontend_app: Option<FrontendApp>,
) -> Result<Config> {
    let mut config = read(env)?;
    config.current_project = Some(project);
    config.current_frontend_app = frontend_app;
    write(env, &config)?;
    Ok(config)
}

/// Selects a tenant for the session. A previously selected project that
/// belongs to a different tenant is no longer in scope, so it (and its
/// frontend app) is cleared along with it.
pub fn set_current_tenant(env: Environment, tenant: Tenant) -> Result<Config> {
    let mut config = read(env)?;

    let project_still_in_scope = config
        .current_project
        .as_ref()
        .and_then(|project| project.tenant_id)
        == Some(tenant.id);
    if !project_still_in_scope {
        config.current_project = None;
        config.current_frontend_app = None;
    }

    config.current_tenant = Some(tenant);
    write(env, &config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use smbcloud_model::tenant::{TenantKind, TenantRole};

    // Regression test: the `toml` crate's deserializer chokes on a
    // `deserialize_with` function that calls `String::deserialize` on a
    // value shaped like a date/time (e.g. the `ar_date_format` helper used
    // elsewhere in this crate for ActiveRecord-style JSON timestamps) —
    // "input contains invalid characters" — even though the same value
    // round-trips fine as a plain `DateTime<Utc>` field. `Tenant` (embedded
    // in `Config`, which this module serializes to TOML) must keep using
    // plain chrono serde for `created_at`, not a custom string format.
    #[test]
    fn config_with_tenant_round_trips_through_toml() {
        let tenant = Tenant {
            id: 1,
            name: "Acme".to_string(),
            slug: "acme".to_string(),
            kind: TenantKind::Organization,
            role: TenantRole::Owner,
            projects_count: 1,
            default_project: None,
            current: false,
            created_at: Utc::now(),
        };
        let config = Config {
            current_tenant: Some(tenant),
            ..Default::default()
        };

        let serialized = toml::to_string(&config).expect("serialize");
        let deserialized: Config = toml::from_str(&serialized).expect("deserialize");

        assert_eq!(
            deserialized.current_tenant.map(|t| t.id),
            config.current_tenant.map(|t| t.id)
        );
    }
}
