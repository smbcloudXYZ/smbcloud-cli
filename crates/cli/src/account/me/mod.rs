use crate::account::lib::is_logged_in;
use crate::client;
use crate::token::get_smb_token::get_smb_token;
use crate::{
    cli::CommandResult,
    ui::{fail_message, fail_symbol, me_view::show_user_tui, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use smbcloud_auth::me::me;

use smbcloud_network::environment::Environment;
use spinners::Spinner;

pub async fn process_me(env: Environment) -> Result<CommandResult> {
    if !is_logged_in(env) {
        return Ok(CommandResult {
            spinner: Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Loading"),
            ),
            symbol: fail_symbol(),
            msg: fail_message("You are not logged in. Please login first."),
        });
    }
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );
    let token = get_smb_token(env)?;
    match me(env, client(), &token).await {
        Ok(user) => {
            spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
            show_user_tui(&user).map_err(|e| anyhow!(e))?;
            Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    succeed_message("Loading"),
                ),
                symbol: succeed_symbol(),
                msg: succeed_message("Done."),
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
