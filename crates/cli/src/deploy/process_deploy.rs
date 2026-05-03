use {
    crate::{
        account::login::process_login,
        cli::CommandResult,
        client,
        deploy::{
            config::{check_project, credentials, get_config},
            detect_runner::detect_runner,
            git::remote_deployment_setup,
            process_deploy_nextjs_ssr::process_deploy_nextjs_ssr,
            process_deploy_rails::process_deploy_rails,
            process_deploy_rust::process_deploy_rust,
            process_deploy_vite_spa::process_deploy_vite_spa,
            remote_messages::{build_next_app, start_server},
            rsync_deploy::rsync_deploy,
        },
        token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in},
        ui::{fail_message, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    dialoguer::{console::Term, theme::ColorfulTheme, Select},
    git2::{PushOptions, RemoteCallbacks, Repository},
    smbcloud_auth::me::me,
    smbcloud_model::{
        project::{DeploymentMethod, DeploymentPayload, DeploymentStatus},
        runner::Runner,
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::{
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

pub async fn process_deploy(
    env: Environment,
    project_name: Option<String>,
) -> Result<CommandResult> {
    // Check credentials.
    let is_logged_in = is_logged_in(env).await?;

    if !is_logged_in {
        let _ = process_login(env, Some(is_logged_in)).await?;
    }

    // Get current token.
    let access_token = get_smb_token(env)?;

    // Check config.
    let mut config = get_config(env, Some(&access_token)).await?;

    // When a sub-project name is given (monorepo), swap config.project to that
    // entry from [[projects]] so all downstream logic operates on the right one.
    // When omitted and the root project is a monorepo, prompt the user to pick.
    let resolved_name = match project_name {
        Some(name) => Some(name),
        None if config.project.runner == Runner::Monorepo => Some(prompt_select_project(&config)?),
        None => None,
    };

    if let Some(ref name) = resolved_name {
        config = resolve_sub_project(config, name)?;
    }

    // Validate that the logged-in user has access to this project before doing
    // any work — applies to every deployment path including vite-spa.
    check_project(env, &access_token, config.project.id).await?;

    // Route Vite SPA projects to a dedicated local-build + rsync deploy path.
    // The kind field in config.toml drives this: kind = "vite-spa".
    if config.project.kind.as_deref() == Some("vite-spa") {
        return process_deploy_vite_spa(env, config).await;
    }

    // Route Next.js SSR projects: pnpm install + build, rsync 8 items, SSH pm2 restart.
    if config.project.kind.as_deref() == Some("nextjs-ssr") {
        return process_deploy_nextjs_ssr(env, config).await;
    }

    // Route Rails projects: rsync shared lib, SSH compile native gem, git force-push sub-project.
    if config.project.kind.as_deref() == Some("rails") {
        return process_deploy_rails(env, config).await;
    }

    // Route Rust service projects: rsync source tree, then run a remote Cargo build script.
    if config.project.kind.as_deref() == Some("rust") {
        return process_deploy_rust(env, config).await;
    }

    match config.project.deployment_method {
        DeploymentMethod::Rsync => {
            // For rsync deployments the runner is known from config — no framework
            // detection needed, the source tree may have no package.json/Gemfile/etc.
            let runner = config.project.runner;
            let user = me(env, client(), &access_token).await?;
            rsync_deploy(&config, &runner, user.id, ".")
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
    let runner = detect_runner(&config).await?;
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

    let payload = DeploymentPayload {
        commit_hash: commit_hash.to_string(),
        status: DeploymentStatus::Started,
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
