pub mod config;
mod git;
mod remote_messages;
mod setup;

use crate::token::get_smb_token;
use crate::{
    account::{lib::is_logged_in, login::process_login},
    cli::CommandResult,
    deploy::config::check_project,
    project::runner::detect_runner,
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use config::check_config;
use git::remote_deployment_setup;
use git2::{PushOptions, RemoteCallbacks, Repository};
use network::environment::Environment;
use remote_messages::{build_next_app, start_server};
use smbcloud_model::project::{DeploymentPayload, DeploymentStatus};
use smbcloud_networking_account::me::me;
use smbcloud_networking_project::{
    crud_project_deployment_create::create_deployment, crud_project_deployment_update::update,
};
use spinners::Spinner;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub async fn process_deploy(env: Environment) -> Result<CommandResult> {
    // Check credentials.
    if !is_logged_in(env) {
        let _ = process_login(env).await;
    }

    // Get current token
    let access_token = get_smb_token(env).await?;

    // Check config.
    let config = check_config(env).await?;

    // Check runner.
    let runner = detect_runner().await?;

    // Validate config with project.
    check_project(env, &access_token, config.project.id).await?;

    // Check remote repository setup.
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(_) => {
            return Err(anyhow!(fail_message(
                "No git repository found. Init with `git init` command."
            )))
        }
    };

    // Get the current branch
    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => {
            return Err(anyhow!(fail_message(
                "No HEAD reference found. Create a commit with `git commit` command."
            )))
        }
    };

    // Check if we're on the main branch
    let branch_name = match head.shorthand() {
        Some(name) => name,
        None => {
            return Err(anyhow!(fail_message(
                "Unable to determine current branch name."
            )))
        }
    };

    if branch_name != "main" && branch_name != "master" {
        return Err(anyhow!(fail_message(
            &format!("Not on main branch. Current branch: '{}'. Switch to main branch with `git checkout main` command.", branch_name)
        )));
    }

    let main_branch = head;

    let repository = match &config.project.repository {
        Some(repo) => repo,
        None => return Err(anyhow!(fail_message("Repository not found."))),
    };

    let mut origin = remote_deployment_setup(&runner, &repo, &repository).await?;

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
        create_deployment(env, &access_token, config.project.id, payload).await?;
    let user = me(env, &access_token).await?;

    let mut push_opts = PushOptions::new();
    let mut callbacks = RemoteCallbacks::new();

    // For updating status to failed
    let deployment_failed_flag = Arc::new(AtomicBool::new(false));
    let update_env = env; // Env is Copy
    let update_access_token = access_token.clone();
    let update_project_id = config.project.id;
    let update_deployment_id = created_deployment.id;

    // Set the credentials
    callbacks.credentials(config.credentials(user));
    callbacks.sideband_progress(|data| {
        // Convert bytes to string, print line by line
        if let Ok(text) = std::str::from_utf8(data) {
            for line in text.lines() {
                if line.contains(&build_next_app()) {
                    println!("Building the app {}", succeed_symbol());
                }
                if line.contains(&start_server(&repository)) {
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
            // Update deployment status to Done
            let update_payload = DeploymentPayload {
                commit_hash: commit_hash.to_string(),
                status: DeploymentStatus::Done,
            };
            let result = update(
                env,
                access_token.clone(),
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
