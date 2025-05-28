pub mod config;
mod git;
mod remote_messages;

use crate::{
    account::lib::protected_request,
    cli::CommandResult,
    deploy::config::check_project,
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use config::check_config;
use git::remote_deployment_setup;
use git2::{PushOptions, RemoteCallbacks, Repository};
use remote_messages::{build_next_app, start_server};
use smbcloud_networking::environment::Environment;
use spinners::Spinner;

pub async fn process_deploy(env: Environment) -> Result<CommandResult> {
    // Check credentials.
    protected_request(env).await?;

    // Check config.
    let config = check_config().await?;

    // Validate config with project.
    check_project(env, config.repository.id).await?;

    // Check remote repository setup.
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(_) => {
            return Err(anyhow!(fail_message(
                "No git repository found. Init with `git init` command."
            )))
        }
    };

    let _main_branch = match repo.head() {
        Ok(branch) => branch,
        Err(_) => {
            return Err(anyhow!(fail_message(
                "No main branch found. Create with `git checkout -b <branch>` command."
            )))
        }
    };

    let mut origin = remote_deployment_setup(&repo, &config.repository.name).await?;

    let mut push_opts = PushOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    // Set the credentials
    callbacks.credentials(config.credentials());
    callbacks.sideband_progress(|data| {
        // Convert bytes to string, print line by line
        if let Ok(text) = std::str::from_utf8(data) {
            for line in text.lines() {
                if line.contains(&build_next_app()) {
                    println!("Building the app {}", succeed_symbol());
                }
                if line.contains(&start_server(&config.repository.name)) {
                    println!("App restart {}", succeed_symbol());
                }
            }
        }
        true // continue receiving.
    });
    callbacks.push_update_reference(|_x, status_message| match status_message {
        Some(e) => {
            println!("Deployment fail: {}", e);
            Ok(())
        }
        None => Ok(()),
    });
    push_opts.remote_callbacks(callbacks);

    let spinner = Spinner::new(
        spinners::Spinners::Hamburger,
        succeed_message("Deploying > "),
    );
    match origin.push(&["refs/heads/main:refs/heads/main"], Some(&mut push_opts)) {
        Ok(_) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Deployment complete."),
        }),
        Err(e) => Err(anyhow!(fail_message(&e.to_string()))),
    }
}
