use {
    crate::{
        account::login::process_login,
        cli::CommandResult,
        client,
        deploy::{
            config::{check_project, credentials, get_config, overlay_server_config},
            git::remote_deployment_setup,
            process_deploy_artifact::{env_token, is_artifact, process_deploy_artifact, TOKEN_ENV},
            process_deploy_astro_ssr::process_deploy_astro_ssr,
            process_deploy_nextjs_ssr::process_deploy_nextjs_ssr,
            process_deploy_rails::process_deploy_rails,
            process_deploy_rust::process_deploy_rust,
            process_deploy_static_site::{process_deploy_sveltekit, process_deploy_vite_spa},
            process_deploy_swift::process_deploy_swift,
            remote_messages::{build_next_app, start_server},
        },
        token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in},
        ui::{fail_message, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    dialoguer::{console::Term, theme::ColorfulTheme, Select},
    git2::{PushOptions, RemoteCallbacks, Repository},
    smbcloud_auth::me::me,
    smbcloud_deploy::Transport,
    smbcloud_model::{
        error_codes::{ErrorCode, ErrorResponse},
        project::{DeploymentMethod, DeploymentPayload, DeploymentStatus},
        runner::Runner,
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::{
        crud_app_deploy_config::get_app_deploy_config,
        crud_project_deployment_create::create_deployment, crud_project_deployment_update::update,
    },
    smbcloud_utils::config::Config,
    spinners::Spinner,
    std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// Interactively prompt the user to pick one of the [[projects]] entries.
fn prompt_select_project(config: &Config) -> Result<String> {
    let projects = config.projects.as_ref().ok_or_else(|| {
        anyhow!(fail_message(
            "No [[projects]] entries found in .smb/config.toml."
        ))
    })?;

    let labels: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();

    // In CI there's no TTY to pick from a monorepo's [[projects]]; the caller
    // must name the target with `--project <name>`.
    if crate::ci::is_ci() {
        return Err(anyhow!(fail_message(&format!(
            "This is a monorepo config. In --ci mode, pass --project <name> to choose which app to deploy. Available: {}",
            labels.join(", ")
        ))));
    }

    let index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select project to deploy")
        .items(&labels)
        .default(0)
        .interact_on(&Term::stderr())
        .map_err(|_| anyhow!(fail_message("No project selected.")))?;

    Ok(labels[index].to_owned())
}

/// Swap `config.project` for the named entry in `config.projects`, preserving
/// the rest of the config unchanged so auth, SSH keys, etc. still resolve.
fn resolve_sub_project(mut config: Config, name: &str) -> Result<Config> {
    let projects = config.projects.as_ref().ok_or_else(|| {
        anyhow!(fail_message(
            "No [[projects]] entries found in .smb/config.toml."
        ))
    })?;

    let sub_project = projects
        .iter()
        .find(|project| project.name == name)
        .ok_or_else(|| {
            anyhow!(fail_message(&format!(
                "Sub-project '{}' not found in [[projects]]. Available: {}",
                name,
                projects
                    .iter()
                    .map(|project| project.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        })?
        .clone();

    config.project = sub_project;
    Ok(config)
}

/// Flags that only the HTTPS artifact path reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeployOptions {
    pub dry_run: bool,
    pub prebuilt: bool,
}

const SSH_DEPRECATION: &str = "note: deploying over SSH is deprecated. Static sites can publish \
    over HTTPS with an app-scoped token (see docs/ci.md).";

/// Resolves the monorepo sub-project, as both auth paths need.
fn select_project(config: Config, project_name: Option<String>) -> Result<Config> {
    let resolved_name = match project_name {
        Some(name) => Some(name),
        None if config.project.runner == Runner::Monorepo => Some(prompt_select_project(&config)?),
        None => None,
    };
    match resolved_name {
        Some(name) => resolve_sub_project(config, &name),
        None => Ok(config),
    }
}

/// Words a failed deploy config read for the `SMB_TOKEN` path. A refused token
/// and a missing app are the two failures a CI job hits after a rotation or a
/// config typo; anything else keeps the generic message.
fn deploy_config_error_message(error: &ErrorResponse, app_id: &str) -> String {
    let ErrorResponse::Error { error_code, .. } = error;
    match error_code {
        ErrorCode::Unauthorized => {
            "The deploy token was refused: it is revoked, expired, or for another app.".to_string()
        }
        ErrorCode::ProjectNotFound => {
            format!("No app {app_id}: check frontend_app_id in .smb/config.toml.")
        }
        _ => format!("Could not read the app's deploy config: {error}"),
    }
}

/// `SMB_TOKEN` carries an app-scoped deploy token. It can reach only the
/// artifact routes of its own app, so there is no `me`, no project check and no
/// SSH key on this path; an app that is not published as an artifact is
/// refused.
async fn process_deploy_with_token(
    env: Environment,
    project_name: Option<String>,
    options: DeployOptions,
    token: String,
) -> Result<CommandResult> {
    let config = get_config(env, Some(&token)).await?;
    let config = select_project(config, project_name)?;
    let app_id = config.project.frontend_app_id.clone().ok_or_else(|| {
        anyhow!(fail_message(
            "The project has no frontend_app_id in .smb/config.toml, so SMB_TOKEN has no app to deploy."
        ))
    })?;
    let remote = get_app_deploy_config(env, client(), &token, &app_id)
        .await
        .map_err(|e| anyhow!(fail_message(&deploy_config_error_message(&e, &app_id))))?;
    if !is_artifact(&remote) {
        return Err(anyhow!(fail_message(&format!(
            "{TOKEN_ENV} deploy tokens only work for apps published over HTTPS \
             (publish_target = artifact), and this app is not."
        ))));
    }

    let mut config = config;
    if let Some(kind) = remote.kind.clone() {
        config.project.kind = Some(kind);
    }
    if let Some(output) = remote.output_path.clone() {
        config.project.output = Some(output);
    }
    if let Some(package_manager) = remote.package_manager.clone() {
        config.project.package_manager = Some(package_manager);
    }
    if let Some(source_path) = remote.source_path.clone() {
        config.project.source_path = Some(source_path);
    }
    process_deploy_artifact(env, config, remote, token, options).await
}

pub async fn process_deploy(
    env: Environment,
    project_name: Option<String>,
    options: DeployOptions,
) -> Result<CommandResult> {
    if let Some(token) = env_token() {
        return process_deploy_with_token(env, project_name, options, token).await;
    }

    // Check credentials.
    let is_logged_in = is_logged_in(env).await?;

    if !is_logged_in {
        // Logging in is interactive; in CI the token must already exist.
        if crate::ci::is_ci() {
            return Err(anyhow!(fail_message(
                "Not authenticated. In --ci mode, log in beforehand (run `smb login` without --ci, \
                 or provision the token at ~/.smb/token) — interactive login is disabled."
            )));
        }
        let _ = process_login(env, Some(is_logged_in)).await?;
    }

    // Get current token.
    let access_token = get_smb_token(env)?;

    // Check config.
    let mut config = get_config(env, Some(&access_token)).await?;

    // When a sub-project name is given (monorepo), swap config.project to that
    // entry from [[projects]] so all downstream logic operates on the right one.
    // When omitted and the root project is a monorepo, prompt the user to pick.
    config = select_project(config, project_name)?;

    overlay_server_config(env, &access_token, &mut config).await;

    // Validate that the logged-in user has access to this project before doing
    // any work — applies to every deployment path including vite-spa.
    check_project(env, &access_token, config.project.id).await?;

    // Apps the server publishes as artifacts deploy over HTTPS: no SSH key,
    // no host pinning, no `me`. Everything else keeps the SSH paths below.
    if let Some(app_id) = config.project.frontend_app_id.clone() {
        if let Ok(remote) = get_app_deploy_config(env, client(), &access_token, &app_id).await {
            if is_artifact(&remote) {
                return process_deploy_artifact(env, config, remote, access_token, options).await;
            }
        }
    }
    eprintln!("{SSH_DEPRECATION}");

    // Route Vite SPA projects to a dedicated local-build + rsync deploy path.
    // The kind field in config.toml drives this: kind = "vite-spa".
    if config.project.kind.as_deref() == Some("vite-spa") {
        return process_deploy_vite_spa(env, config).await;
    }

    // Route SvelteKit projects built with adapter-static: install + build, rsync the output.
    if config.project.kind.as_deref() == Some("sveltekit") {
        return process_deploy_sveltekit(env, config).await;
    }

    // Route Next.js SSR projects: pnpm install + build, rsync 8 items, SSH pm2 restart.
    if config.project.kind.as_deref() == Some("nextjs-ssr") {
        return process_deploy_nextjs_ssr(env, config).await;
    }

    // Route Astro SSR projects (@astrojs/node standalone): build locally, rsync dist/, SSH install + systemd restart.
    if config.project.kind.as_deref() == Some("astro-ssr") {
        return process_deploy_astro_ssr(env, config).await;
    }

    // Route Rails projects: rsync shared lib, SSH compile native gem, git force-push sub-project.
    if config.project.kind.as_deref() == Some("rails") {
        return process_deploy_rails(env, config).await;
    }

    // Route Rust service projects: build a Linux binary locally, upload it over rsync, then restart it over SSH.
    if config.project.kind.as_deref() == Some("rust") {
        return process_deploy_rust(env, config).await;
    }

    // Route Swift/Vapor projects: build a Linux binary via Docker, rsync binary + Resources/ + Public/, SSH restart.
    if config.project.kind.as_deref() == Some("swift") {
        return process_deploy_swift(env, config).await;
    }

    match config.project.deployment_method {
        DeploymentMethod::Rsync => {
            // For rsync deployments the runner is known from config — no framework
            // detection needed, the source tree may have no package.json/Gemfile/etc.
            let runner = config.project.runner;
            let user = me(env, client(), &access_token).await?;
            let transport = crate::deploy::rsync_transport(&config, &runner, user.id)?;

            // The engine ships silently; this command owns the spinner and the
            // final line it hands back to `main` to persist.
            let spinner = Spinner::new(
                spinners::Spinners::Hamburger,
                succeed_message(&format!("Syncing to {}…", runner.rsync_host())),
            );
            match transport.ship(std::path::Path::new("."), &smbcloud_deploy::NoopReporter) {
                Ok(()) => Ok(CommandResult {
                    spinner,
                    symbol: succeed_symbol(),
                    msg: succeed_message("Deployment complete via rsync."),
                }),
                Err(e) => {
                    drop(spinner);
                    Err(anyhow!(fail_message(&format!("{e}"))))
                }
            }
        }
        DeploymentMethod::Git => git_deploy(env, &access_token, config).await,
    }
}

async fn git_deploy(
    env: Environment,
    access_token: &str,
    config: smbcloud_utils::config::Config,
) -> Result<CommandResult> {
    // Runner detection requires framework files (package.json, Gemfile, etc.) —
    // only needed for the git push path where the server builds the project.
    let reporter = crate::ui::reporter::SpinnerReporter::new();
    let runner = smbcloud_deploy::detect_runner(&config, &reporter)?;
    // Check remote repository setup.
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(_) => {
            return Err(anyhow!(fail_message(
                "No git repository found. Init with `git init` command."
            )))
        }
    };

    // Get the current branch.
    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => {
            return Err(anyhow!(fail_message(
                "No HEAD reference found. Create a commit with `git commit` command."
            )))
        }
    };

    // Check if we're on the main branch.
    let branch_name = match head.shorthand() {
        Some(name) => name,
        None => {
            return Err(anyhow!(fail_message(
                "Unable to determine current branch name."
            )))
        }
    };

    if branch_name != "main" && branch_name != "master" {
        return Err(anyhow!(fail_message(&format!(
            "Not on main branch. Current branch: '{}'. Switch to main branch with `git checkout main` command.",
            branch_name
        ))));
    }

    let main_branch = head;

    let repository = match &config.project.repository {
        Some(repo) => repo,
        None => return Err(anyhow!(fail_message("Repository not found."))),
    };

    let mut origin = remote_deployment_setup(&runner, &repo, repository).await?;

    let commit_hash = match main_branch.resolve() {
        Ok(result) => match result.target() {
            Some(hash_id) => hash_id,
            None => return Err(anyhow!("Should have at least one commit.")),
        },
        Err(_) => return Err(anyhow!("Cannot resolve main branch.")),
    };

    let frontend_app_id = config.project.frontend_app_id.clone();

    let payload = DeploymentPayload {
        commit_hash: commit_hash.to_string(),
        status: DeploymentStatus::Started,
        frontend_app_id: frontend_app_id.clone(),
    };

    let created_deployment =
        create_deployment(env, client(), access_token, config.project.id, payload).await?;
    let user = me(env, client(), access_token).await?;

    let mut push_opts = PushOptions::new();
    let mut callbacks = RemoteCallbacks::new();

    // For updating status to failed.
    let deployment_failed_flag = Arc::new(AtomicBool::new(false));
    let update_env = env; // Env is Copy
    let update_access_token = access_token.to_owned();
    let update_project_id = config.project.id;
    let update_deployment_id = created_deployment.id;

    // Set the credentials.
    callbacks.credentials(credentials(&config, user));
    callbacks.sideband_progress(|data| {
        if let Ok(text) = std::str::from_utf8(data) {
            for line in text.lines() {
                if line.contains(&build_next_app()) {
                    println!("Building the app {}", succeed_symbol());
                }
                if line.contains(&start_server(repository)) {
                    println!("App restart {}", succeed_symbol());
                }
            }
        }
        true // continue receiving.
    });
    callbacks.push_update_reference({
        let flag_clone = deployment_failed_flag.clone();
        let access_token_for_update_cb = update_access_token.clone();
        let project_id_for_update_cb = update_project_id;
        let deployment_id_for_update_cb = update_deployment_id;
        let frontend_app_id_for_update_cb = frontend_app_id.clone();

        move |_refname, status_message| {
            if let Some(e) = status_message {
                // Try to set the flag. If it was already true, do nothing.
                if !flag_clone.swap(true, Ordering::SeqCst) {
                    println!(
                        "Deployment ref update failed: {}. Marking deployment as Failed.",
                        e
                    );

                    let update_payload = DeploymentPayload {
                        commit_hash: commit_hash.to_string(),
                        status: DeploymentStatus::Failed,
                        frontend_app_id: frontend_app_id_for_update_cb.clone(),
                    };

                    // We are in a sync callback, so we need to block on the async task.
                    let handle = tokio::runtime::Handle::current();
                    let result = handle.block_on(async {
                        update(
                            update_env, // Env is Copy
                            client(),
                            access_token_for_update_cb.clone(),
                            project_id_for_update_cb,
                            deployment_id_for_update_cb,
                            update_payload,
                        )
                        .await
                    });

                    match result {
                        Ok(_) => println!("Deployment status successfully updated to Failed."),
                        Err(update_err) => {
                            eprintln!("Error updating deployment status to Failed: {}", update_err)
                        }
                    }
                }
            }
            Ok(()) // Report success for the git callback itself, error is handled above.
        }
    });
    push_opts.remote_callbacks(callbacks);

    let spinner = Spinner::new(
        spinners::Spinners::Hamburger,
        succeed_message("Deploying > "),
    );

    match origin.push(&["refs/heads/main:refs/heads/main"], Some(&mut push_opts)) {
        Ok(_) => {
            // Update deployment status to Done.
            let update_payload = DeploymentPayload {
                commit_hash: commit_hash.to_string(),
                status: DeploymentStatus::Done,
                frontend_app_id: frontend_app_id.clone(),
            };
            let result = update(
                env,
                client(),
                access_token.to_owned(),
                config.project.id,
                created_deployment.id,
                update_payload,
            )
            .await;
            match result {
                Ok(_) => println!("App is running {}", succeed_symbol()),
                Err(update_err) => {
                    eprintln!("Error updating deployment status to Done: {}", update_err)
                }
            }
            Ok(CommandResult {
                spinner,
                symbol: succeed_symbol(),
                msg: succeed_message("Deployment complete."),
            })
        }
        Err(e) => Err(anyhow!(fail_message(&e.to_string()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(error_code: ErrorCode) -> ErrorResponse {
        ErrorResponse::Error {
            error_code,
            message: String::new(),
        }
    }

    #[test]
    fn refused_token_message() {
        assert_eq!(
            deploy_config_error_message(&error(ErrorCode::Unauthorized), "7"),
            "The deploy token was refused: it is revoked, expired, or for another app."
        );
    }

    #[test]
    fn missing_app_message() {
        assert_eq!(
            deploy_config_error_message(&error(ErrorCode::ProjectNotFound), "7"),
            "No app 7: check frontend_app_id in .smb/config.toml."
        );
    }

    #[test]
    fn other_errors_keep_the_generic_message() {
        let message = deploy_config_error_message(&error(ErrorCode::NetworkError), "7");
        assert!(message.starts_with("Could not read the app's deploy config:"));
    }
}
