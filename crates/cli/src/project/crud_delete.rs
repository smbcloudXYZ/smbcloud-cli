use crate::{
    cli::CommandResult,
    ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
};
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use smbcloud_networking::environment::Environment;
use smbcloud_networking_project::delete_project;
use spinners::Spinner;

pub async fn process_project_delete(env: Environment, id: String) -> Result<CommandResult> {
    let confirmation = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Are you sure you want to delete this project? (y/n)")
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
