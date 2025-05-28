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
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct ProjectRow {
    #[tabled(rename = "ID")]
    id: i32,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Description")]
    description: String,
    #[tabled(rename = "Created at")]
    created_at: String,
    #[tabled(rename = "Updated at")]
    updated_at: String,
}

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

            let spinner = Spinner::new(
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
                Ok(_) => Ok(CommandResult {
                    spinner,
                    symbol: succeed_symbol(),
                    msg: succeed_message("Done. Project has been deleted."),
                }),
                Err(e) => Ok(CommandResult {
                    spinner,
                    symbol: fail_symbol(),
                    msg: fail_message(&e.to_string()),
                }),
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
    if projects.is_empty() {
        return;
    }
    let rows: Vec<ProjectRow> = projects
        .into_iter()
        .map(|p| ProjectRow {
            id: p.id,
            name: p.name,
            description: p.description,
            created_at: p.created_at.date_naive().to_string(),
            updated_at: p.updated_at.date_naive().to_string(),
        })
        .collect();
    let table = Table::new(rows);
    println!("{table}");
}
