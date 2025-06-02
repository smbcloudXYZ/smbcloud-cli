pub mod config;
mod git;
mod remote_messages;
mod setup;

use crate::{
    account::{lib::is_logged_in, login::process_login, me::me},
    cli::CommandResult,
    deploy::config::check_project,
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use config::check_config;
use git::remote_deployment_setup;
use git2::{PushOptions, RemoteCallbacks, Repository};
use remote_messages::{build_next_app, start_server};
use smbcloud_model::project::{DeploymentPayload, DeploymentStatus};
use smbcloud_networking::{environment::Environment, get_smb_token};
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

    let main_branch = match repo.head() {
        Ok(branch) => branch,
        Err(_) => {
            return Err(anyhow!(fail_message(
                "No main branch found. Create with `git checkout -b <branch>` command."
            )))
        }
    };

    let mut origin = remote_deployment_setup(&repo, &config.project.repository).await?;

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
    let user = me(env).await?;

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
                if line.contains(&start_server(&config.project.repository)) {
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
                Ok(_) => println!("Deployment status successfully updated to Done."),
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
