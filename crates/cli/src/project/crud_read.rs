use std::{fs::OpenOptions, io::Write};

use crate::{
    cli::CommandResult,
    ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use log::debug;
use smbcloud_model::project::{Config, Project};
use smbcloud_networking::{environment::Environment, get_smb_token};
use smbcloud_networking_project::{crud_project_deployment_read::get_project, get_all};
use spinners::Spinner;
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct ProjectRow {
    #[tabled(rename = "ID")]
    id: i32,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Repository")]
    repository: String,
    #[tabled(rename = "Description")]
    description: String,
    #[tabled(rename = "Created at")]
    created_at: String,
    #[tabled(rename = "Updated at")]
    updated_at: String,
}

pub async fn process_project_list(env: Environment) -> Result<CommandResult> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );

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

pub async fn process_project_show(env: Environment, id: String) -> Result<CommandResult> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );
    let access_token = get_smb_token(env).await?;
    match get_project(env, access_token, id).await {
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

pub(crate) fn show_projects(projects: Vec<Project>) {
    if projects.is_empty() {
        return;
    }
    let rows: Vec<ProjectRow> = projects
        .into_iter()
        .map(|p| ProjectRow {
            id: p.id,
            name: p.name,
            repository: p.repository,
            description: p.description.unwrap_or("-".to_owned()),
            created_at: p.created_at.date_naive().to_string(),
            updated_at: p.updated_at.date_naive().to_string(),
        })
        .collect();
    let table = Table::new(rows);
    println!("{table}");
}

pub(crate) async fn process_project_use(env: Environment, id: String) -> Result<CommandResult> {
    let access_token = get_smb_token(env).await?;
    let project = get_project(env, access_token, id).await?;

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
