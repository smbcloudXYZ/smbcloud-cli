use crate::client;
use crate::token::get_smb_token::get_smb_token;
use crate::{
    account::lib::is_logged_in,
    cli::CommandResult,
    ui::{
        confirm_dialog::confirm_delete_tui, fail_message, fail_symbol, succeed_message,
        succeed_symbol,
    },
};
use anyhow::{anyhow, Result};
use smbcloud_network::environment::Environment;
use smbcloud_networking_project::crud_project_delete::delete_project;
use spinners::Spinner;

pub async fn process_project_delete(env: Environment, id: String) -> Result<CommandResult> {
    if !is_logged_in(env) {
        return Err(anyhow!(fail_message("Please log in with `smb init`.")));
    }

    let confirmed = confirm_delete_tui(&format!("Delete project #{id}")).map_err(|e| anyhow!(e))?;

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
    let access_token = get_smb_token(env)?;
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
