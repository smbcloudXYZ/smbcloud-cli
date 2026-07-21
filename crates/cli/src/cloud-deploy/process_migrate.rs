use {
    crate::{
        account::login::process_login,
        cli::CommandResult,
        client,
        deploy::config::get_config,
        token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in},
        ui::{fail_message, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    smbcloud_model::{
        deploy_config_update::DeployConfigUpdate,
        project::{DeploymentMethod, Project},
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::crud_frontend_app_update_deploy_config::update_deploy_config,
    smbcloud_utils::{config::Config, write_config::write_config},
    spinners::Spinner,
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
        process::Command,
    },
};

const ECOSYSTEM_ENV_EXTRACTOR: &str = r#"
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const ecosystemPath = path.resolve(process.argv[1]);
const pm2AppName = process.argv[2] || '';
const source = fs.readFileSync(ecosystemPath, 'utf8');
const moduleRef = { exports: {} };
const sandbox = {
  module: moduleRef,
  exports: moduleRef.exports,
  require,
  process,
  console,
  __dirname: path.dirname(ecosystemPath),
  __filename: ecosystemPath,
};
sandbox.global = sandbox;
sandbox.globalThis = sandbox;

vm.runInNewContext(source, sandbox, { filename: ecosystemPath });

const loadedConfig = moduleRef.exports;
const apps = Array.isArray(loadedConfig && loadedConfig.apps) ? loadedConfig.apps : [];
let selectedApp = null;

if (pm2AppName) {
  selectedApp = apps.find((app) => app && app.name === pm2AppName) || null;
}

if (!selectedApp && apps.length === 1) {
  selectedApp = apps[0];
}

const envProduction = selectedApp && typeof selectedApp.env_production === 'object'
  ? selectedApp.env_production
  : null;

process.stdout.write(JSON.stringify(envProduction));
"#;

fn ecosystem_config_path(project: &Project) -> Option<PathBuf> {
    let source_directory = project
        .source
        .as_deref()
        .or(project.source_path.as_deref())
        .unwrap_or(".");
    let source_directory = PathBuf::from(source_directory);

    let ecosystem_config_cjs = source_directory.join("ecosystem.config.cjs");
    if ecosystem_config_cjs.is_file() {
        return Some(ecosystem_config_cjs);
    }

    let ecosystem_config_js = source_directory.join("ecosystem.config.js");
    if ecosystem_config_js.is_file() {
        return Some(ecosystem_config_js);
    }

    None
}

fn read_pm2_env_from_ecosystem_file(
    ecosystem_path: &Path,
    pm2_app_name: Option<&str>,
) -> Result<Option<HashMap<String, serde_json::Value>>> {
    let output = Command::new("node")
        .arg("-e")
        .arg(ECOSYSTEM_ENV_EXTRACTOR)
        .arg(ecosystem_path)
        .arg(pm2_app_name.unwrap_or(""))
        .output()
        .map_err(|error| {
            anyhow!(
                "Failed to run `node` to read '{}': {}",
                ecosystem_path.display(),
                error
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow!(
            "Failed to parse '{}': {}",
            ecosystem_path.display(),
            if stderr.is_empty() {
                "node exited without error output".to_string()
            } else {
                stderr
            }
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() || stdout == "null" {
        return Ok(None);
    }

    serde_json::from_str(&stdout).map(Some).map_err(|error| {
        anyhow!(
            "Failed to decode env vars from '{}': {}",
            ecosystem_path.display(),
            error
        )
    })
}

/// Build a `DeployConfigUpdate` from the local fields on a `Project`.
/// Returns `None` when there is nothing to migrate.
fn build_payload(
    project: &Project,
    pm2_env: Option<HashMap<String, serde_json::Value>>,
) -> Option<DeployConfigUpdate> {
    // runner and deployment_method are always present (defaulted), but we only
    // want to send them when they differ from the zero-value defaults, or when
    // any other field is populated. Simpler: always include them so the server
    // has the full picture.
    let runner = Some(project.runner as u8);
    let deployment_method = Some(project.deployment_method as u8);

    let payload = DeployConfigUpdate {
        runner,
        deployment_method,
        kind: project.kind.clone(),
        // `source` is the local build directory; maps to source_path on server.
        // Fall back to `source_path` if `source` is absent.
        source_path: project
            .source
            .clone()
            .or_else(|| project.source_path.clone()),
        remote_path: project.path.clone(),
        package_manager: project.package_manager.clone(),
        pm2_app: project.pm2_app.clone(),
        pm2_env,
        port: project.port,
        output_path: project.output.clone(),
        build_command: project.compile_cmd.clone(),
        install_command: project.install_command.clone(),
        binary_name: project.binary_name.clone(),
        build_target: project.rust_target.clone(),
        shared_lib_path: project.shared_lib.clone(),
    };

    // runner and deployment_method are always Some, so the payload is never
    // technically "empty" — but if every optional field is None and the runner
    // and deployment_method are their defaults, there is nothing interesting to
    // push. We still send it so the server record is always consistent.
    Some(payload)
}

/// Strip all deploy fields from a `Project`, keeping only identity fields.
fn strip_project(project: &Project) -> Project {
    Project {
        id: project.id,
        tenant_id: project.tenant_id,
        name: project.name.clone(),
        runner: project.runner,
        deployment_method: DeploymentMethod::default(),
        frontend_app_id: project.frontend_app_id.clone(),
        description: project.description.clone(),
        // Everything below is intentionally cleared.
        path: None,
        repository: None,
        deploy_repo_id: None,
        source_path: None,
        created_at: project.created_at,
        updated_at: project.updated_at,
        kind: None,
        source: None,
        output: None,
        package_manager: None,
        pm2_app: None,
        pm2_env: None,
        port: None,
        shared_lib: None,
        compile_cmd: None,
        install_command: None,
        binary_name: None,
        rust_target: None,
        swift_sdk: None,
        swift_toolchain: None,
    }
}

pub async fn process_migrate(env: Environment) -> Result<CommandResult> {
    let is_logged_in = is_logged_in(env).await?;
    if !is_logged_in {
        let _ = process_login(env, Some(is_logged_in)).await?;
    }

    let access_token = get_smb_token(env)?;
    let config = get_config(env, Some(&access_token))
        .await
        .map_err(|e| anyhow!(fail_message(&format!("Failed to load config: {:?}", e))))?;

    // Collect all projects to consider: root project + all [[projects]] entries.
    let mut candidates: Vec<Project> = vec![config.project.clone()];
    if let Some(ref sub_projects) = config.projects {
        candidates.extend(sub_projects.iter().cloned());
    }

    let to_migrate: Vec<Project> = candidates
        .into_iter()
        .filter(|project| project.frontend_app_id.is_some())
        .collect();

    if to_migrate.is_empty() {
        let spinner = Spinner::new(spinners::Spinners::Hamburger, String::new());
        return Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("No projects with a frontend_app_id found — nothing to migrate."),
        });
    }

    let spinner = Spinner::new(
        spinners::Spinners::Hamburger,
        succeed_message("Migrating deploy config to server..."),
    );

    let mut migrated = 0usize;
    let mut failed = 0usize;

    for project in &to_migrate {
        let frontend_app_id = project.frontend_app_id.as_deref().unwrap();

        println!(
            "\n  Migrating '{}' (frontend_app_id: {})...",
            project.name, frontend_app_id
        );

        let pm2_env = match ecosystem_config_path(project) {
            Some(ecosystem_path) => {
                match read_pm2_env_from_ecosystem_file(&ecosystem_path, project.pm2_app.as_deref())
                {
                    Ok(Some(env_values)) => {
                        println!(
                            "    {} Migrating {} env vars from '{}'.",
                            succeed_symbol(),
                            env_values.len(),
                            ecosystem_path.display()
                        );
                        Some(env_values)
                    }
                    Ok(None) => {
                        println!(
                            "    {} No env_production block found in '{}'.",
                            succeed_symbol(),
                            ecosystem_path.display()
                        );
                        None
                    }
                    Err(error) => {
                        println!(
                            "    {} Failed to read env vars from '{}': {}",
                            fail_message("✘"),
                            ecosystem_path.display(),
                            error
                        );
                        None
                    }
                }
            }
            None => None,
        };

        let payload = match build_payload(project, pm2_env) {
            Some(p) => p,
            None => {
                println!("    {} Nothing to migrate — skipping.", succeed_symbol());
                continue;
            }
        };

        match update_deploy_config(
            env,
            client(),
            access_token.clone(),
            frontend_app_id,
            &payload,
        )
        .await
        {
            Ok(_) => {
                println!("    {} Migrated successfully.", succeed_symbol());
                migrated += 1;
            }
            Err(e) => {
                println!("    {} Failed: {:?}", fail_message("✘"), e);
                failed += 1;
            }
        }
    }

    // Rewrite config with all deploy fields stripped.
    let stripped_root = strip_project(&config.project);
    let stripped_projects = config
        .projects
        .as_ref()
        .map(|sub_projects| sub_projects.iter().map(strip_project).collect::<Vec<_>>());

    let stripped_config = Config {
        name: config.name.clone(),
        description: config.description.clone(),
        project: stripped_root,
        projects: stripped_projects,
    };

    if let Err(e) = write_config(".", stripped_config) {
        println!(
            "\n  {} Failed to rewrite .smb/config.toml: {:?}",
            fail_message("✘"),
            e
        );
    } else {
        println!(
            "\n  {} .smb/config.toml rewritten (identity-only).",
            succeed_symbol()
        );
    }

    let summary = format!(
        "Migration complete: {} migrated, {} failed.",
        migrated, failed
    );

    Ok(CommandResult {
        spinner,
        symbol: succeed_symbol(),
        msg: succeed_message(&summary),
    })
}
