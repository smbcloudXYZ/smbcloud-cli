pub mod cli;
pub mod init;

use self::cli::Commands;
use crate::{
    cli::CommandResult,
    ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Input};
use init::process_project_init;
use log::debug;
use smbcloud_model::project::{Config, Project};
use smbcloud_networking::environment::Environment;
use smbcloud_networking_project::{delete_project, get_all, get_project};
use spinners::Spinner;
use std::{fs::OpenOptions, io::Write};

pub async fn process_project(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::New {} => process_project_init(env).await,
        Commands::List {} => {
            let mut spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Loading"),
            );

            // Get all
            match get_all(env).await {
                Ok(projects) => {
                    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
                    let msg = if projects.is_empty() {
                        succeed_message("No projects found.")
                    } else {
                        succeed_message("Showing all projects.")
                    };
                    show_projects(projects);
                    Ok(CommandResult {
                        spinner: Spinner::new(
                            spinners::Spinners::SimpleDotsScrolling,
                            succeed_message("Loading"),
                        ),
                        symbol: succeed_symbol(),
                        msg,
                    })
                }
                Err(e) => {
                    println!("Error: {e:#?}");
                    Ok(CommandResult {
                        spinner,
                        symbol: fail_symbol(),
                        msg: fail_message("Failed to get all projects."),
                    })
                }
            }
        }
        Commands::Show { id } => {
            let mut spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Loading"),
            );
            // Get Detail
            match get_project(env, id).await {
                Ok(project) => {
                    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
                    let message = succeed_message(&format!("Showing project {}.", &project.name));
                    show_projects(vec![project]);
                    Ok(CommandResult {
                        spinner: Spinner::new(
                            spinners::Spinners::SimpleDotsScrolling,
                            succeed_message("Loading"),
                        ),
                        symbol: succeed_symbol(),
                        msg: message,
                    })
                }
                Err(e) => {
                    spinner.stop_and_persist(&fail_symbol(), fail_message("Failed."));
                    Err(anyhow!("{e}"))
                }
            }
        }
        Commands::Delete { id } => {
            let confirmation = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure? (y/n)")
                .interact()
                .unwrap();

            let mut spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Deleting project"),
            );

            if confirmation != "y" {
                return Ok(CommandResult {
                    spinner,
                    symbol: succeed_symbol(),
                    msg: succeed_message("Cancelled."),
                });
            }
            match delete_project(env, id).await {
                Ok(_) => {
                    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Done."));
                    Ok(CommandResult {
                        spinner: Spinner::new(
                            spinners::Spinners::SimpleDotsScrolling,
                            succeed_message("Loading"),
                        ),
                        symbol: succeed_symbol(),
                        msg: succeed_message("Project has been deleted."),
                    })
                }
                Err(e) => {
                    spinner.stop_and_persist(&fail_symbol(), fail_message("Failed."));
                    Err(anyhow!("{e}"))
                }
            }
        }
        Commands::Use { id } => {
            let project = get_project(env, id).await?;

            let config = Config {
                current_project: Some(project),
                current_auth_app: None,
            };

            let spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Loading"),
            );
            match home::home_dir() {
                Some(path) => {
                    debug!("{}", path.to_str().unwrap());
                    let mut file = OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .write(true)
                        .open([path.to_str().unwrap(), "/.smb/config.json"].join(""))?;
                    let json = serde_json::to_string(&config)?;
                    file.write_all(json.as_bytes())?;

                    Ok(CommandResult {
                        spinner,
                        symbol: succeed_symbol(),
                        msg: succeed_message("Use project successful."),
                    })
                }
                None => {
                    let error = anyhow!("Failed to get home directory.");
                    Err(error)
                }
            }
        }
    }
}

// Private functions

fn show_projects(projects: Vec<Project>) {
    // println!("Projects: {projects:#?}");
    if projects.is_empty() {
        return;
    }
    println!(
        "{0: <5} | {1: <20} | {2: <30} | {3: <20} | {4: <20}",
        "ID", "Name", "Description", "Created at", "Updated at"
    );
    for project in projects {
        println!(
            "{0: <5} | {1: <20} | {2: <30} | {3: <20} | {4: <20}",
            project.id,
            project.name,
            project.description,
            project.created_at.date_naive(),
            project.updated_at.date_naive(),
        );
    }
}
