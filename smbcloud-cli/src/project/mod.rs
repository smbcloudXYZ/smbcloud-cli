pub mod cli;
pub mod init;

use self::cli::Commands;
use crate::cli::CommandResult;
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use log::debug;
use smbcloud_model::{
    self,
    project::{Config, Project, ProjectCreate},
};
use smbcloud_networking::environment::Environment;
use smbcloud_networking_project::{create_project, delete_project, get_all, get_project};
use spinners::Spinner;
use std::{fs::OpenOptions, io::Write};

pub async fn process_project(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::New {} => {
            let project_name = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Project name")
                .interact()
                .unwrap();
            let description = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Description")
                .interact()
                .unwrap();

            let mut spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Creating a project...").green().bold().to_string(),
            );

            match create_project(
                env,
                ProjectCreate {
                    name: project_name.clone(),
                    description: description.clone(),
                },
            )
            .await
            {
                Ok(_) => {
                    spinner.stop_and_persist("✅", "Done.".to_owned());
                    Ok(CommandResult {
                        spinner: Spinner::new(
                            spinners::Spinners::SimpleDotsScrolling,
                            style("Loading...").green().bold().to_string(),
                        ),
                        symbol: "✅".to_owned(),
                        msg: format!("{project_name} has been created."),
                    })
                }
                Err(e) => {
                    println!("Error: {e:#?}");
                    Ok(CommandResult {
                        spinner,
                        symbol: "😩".to_owned(),
                        msg: format!("Failed to create a project {project_name}."),
                    })
                }
            }
        }
        Commands::List {} => {
            let mut spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Loading...").green().bold().to_string(),
            );

            // Get all
            match get_all(env).await {
                Ok(projects) => {
                    spinner.stop_and_persist("✅", "Loaded.".to_owned());
                    let msg = if projects.is_empty() {
                        "No projects found.".to_owned()
                    } else {
                        "Showing all projects.".to_owned()
                    };
                    show_projects(projects);
                    Ok(CommandResult {
                        spinner: Spinner::new(
                            spinners::Spinners::SimpleDotsScrolling,
                            style("Loading...").green().bold().to_string(),
                        ),
                        symbol: "✅".to_owned(),
                        msg,
                    })
                }
                Err(e) => {
                    println!("Error: {e:#?}");
                    Ok(CommandResult {
                        spinner,
                        symbol: "😩".to_owned(),
                        msg: "Failed to get all projects.".to_owned(),
                    })
                }
            }
        }
        Commands::Show { id } => {
            let mut spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Loading...").green().bold().to_string(),
            );
            // Get Detail
            match get_project(env, id).await {
                Ok(project) => {
                    spinner.stop_and_persist("✅", "Loaded.".to_owned());
                    let message = format!("Showing project {}.", &project.name);
                    show_projects(vec![project]);
                    Ok(CommandResult {
                        spinner: Spinner::new(
                            spinners::Spinners::SimpleDotsScrolling,
                            style("Loading...").green().bold().to_string(),
                        ),
                        symbol: "✅".to_owned(),
                        msg: message,
                    })
                }
                Err(e) => {
                    spinner.stop_and_persist("😩", "Failed.".to_string());
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
                style("Deleting project...").green().bold().to_string(),
            );

            if confirmation != "y" {
                return Ok(CommandResult {
                    spinner,
                    symbol: "✅".to_owned(),
                    msg: "Cancelled.".to_string(),
                });
            }
            match delete_project(env, id).await {
                Ok(_) => {
                    spinner.stop_and_persist("✅", "Done.".to_string());
                    Ok(CommandResult {
                        spinner: Spinner::new(
                            spinners::Spinners::SimpleDotsScrolling,
                            style("Loading...").green().bold().to_string(),
                        ),
                        symbol: "✅".to_owned(),
                        msg: "Project has been deleted.".to_string(),
                    })
                }
                Err(e) => {
                    spinner.stop_and_persist("😩", "Failed.".to_string());
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
                style("Loading...").green().bold().to_string(),
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
                        symbol: "✅".to_owned(),
                        msg: "Use project successful.".to_string(),
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
