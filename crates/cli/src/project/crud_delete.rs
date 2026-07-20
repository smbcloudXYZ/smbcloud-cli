use crate::client;
use crate::token::get_smb_token::get_smb_token;
use crate::{
    account::lib::is_logged_in,
    cli::CommandResult,
    ui::{fail_message, fail_symbol, prompt::confirm_delete_typed, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use smbcloud_network::environment::Environment;
use smbcloud_networking_project::{
    crud_project_delete::delete_project, crud_project_read::get_project,
};
use spinners::Spinner;

pub async fn process_project_delete(env: Environment, id: String) -> Result<CommandResult> {
    if !is_logged_in(env) {
        return Err(anyhow!(fail_message("Please log in with `smb init`.")));
    }

    let access_token = get_smb_token(env)?;
    let project = get_project(env, client(), access_token.clone(), id.clone()).await?;

    let confirmed = confirm_delete_typed(
        "Project deletion confirmation",
        &format!(
            "This permanently deletes project #{} and everything under it — repos, apps, and deployment history.",
            project.id
        ),
        &project.name,
    )?;

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Deleting project"),
    );

    if !confirmed {
        return Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Cancelled."),
        });
    }
    match delete_project(env, client(), access_token, id).await {
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
