use {
    crate::{
        cli::CommandResult,
        client,
        token::{get_smb_token::get_smb_token, smb_token_file_path::smb_token_file_path},
        ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    dialoguer::{theme::ColorfulTheme, Confirm},
    smbcloud_network::environment::Environment,
    smbcloud_networking_account::logout::logout,
    spinners::Spinner,
    std::fs,
};

pub async fn process_logout(env: Environment) -> Result<CommandResult> {
    // Logout if user confirms
    if let Some(token_path) = smb_token_file_path(env) {
        let confirm = match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to logout? y/n")
            .interact()
        {
            Ok(confirm) => confirm,
            Err(_) => {
                let error = anyhow!("Invalid input.");
                return Err(error);
            }
        };
        if !confirm {
            return Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    succeed_message("Cancel operation"),
                ),
                symbol: succeed_symbol(),
                msg: succeed_message("Doing nothing."),
            });
        }

        let spinner = Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            succeed_message("Logging you out"),
        );

        // Call backend
        match do_process_logout(env).await {
            Ok(_) => {
                fs::remove_file(token_path)?;
                Ok(CommandResult {
                    spinner,
                    symbol: succeed_symbol(),
                    msg: succeed_message("You are logged out!"),
                })
            }
            Err(e) => Err(anyhow!("{e}")),
        }
    } else {
        Ok(CommandResult {
            spinner: Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Loading"),
            ),
            symbol: fail_symbol(),
            msg: fail_message("You are not logged in. Please login first."),
        })
    }
}

async fn do_process_logout(env: Environment) -> Result<()> {
    let token = get_smb_token(env)?;
    match logout(env, client(), token).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("{e}")),
    }
}
