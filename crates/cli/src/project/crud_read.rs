use std::{fs::OpenOptions, io::Write};

use crate::client;
use crate::token::get_smb_token::get_smb_token;
use crate::{
    cli::CommandResult,
    ui::{
        fail_message, fail_symbol, project_detail_view::show_project_detail_tui,
        project_table::show_projects_tui, succeed_message, succeed_symbol,
    },
};
use anyhow::{anyhow, Result};
use log::debug;
use smbcloud_model::project::{Config, Project};
use smbcloud_network::environment::Environment;
use smbcloud_networking_project::crud_project_read::{get_project, get_projects};
use spinners::Spinner;

pub async fn process_project_list(env: Environment) -> Result<CommandResult> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );
    let token = get_smb_token(env)?;
    match get_projects(env, client(), token).await {
        Ok(projects) => {
            spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
            let msg = if projects.is_empty() {
                succeed_message("No projects found.")
            } else {
                succeed_message("Done.")
            };
            show_projects(projects)?;
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
    let access_token = get_smb_token(env)?;
    match get_project(env, client(), access_token, id).await {
        Ok(project) => {
            spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
            let message = succeed_message(&format!("Showing project {}.", &project.name));
            show_project_detail(&project)?;
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

pub(crate) fn show_projects(projects: Vec<Project>) -> Result<()> {
    if projects.is_empty() {
        return Ok(());
    }
    show_projects_tui(projects).map_err(|e| anyhow!(e))
}

pub(crate) fn show_project_detail(project: &Project) -> Result<()> {
    show_project_detail_tui(project).map_err(|e| anyhow!(e))
}

pub(crate) async fn process_project_use(env: Environment, id: String) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let project = get_project(env, client(), access_token, id).await?;

    let config = Config {
        current_project: Some(project),
        current_frontend_app: None,
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
