use {
    crate::{
        cli::CommandResult,
        client,
        token::{get_smb_token::get_smb_token, smb_token_file_path::smb_token_file_path},
        ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    dialoguer::{theme::ColorfulTheme, Confirm},
    smbcloud_auth::logout::logout,
    smbcloud_network::environment::Environment,
    spinners::Spinner,
    std::fs,
};

pub async fn process_logout(env: Environment) -> Result<CommandResult> {
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
                    succeed_message("Cancelled."),
                ),
                symbol: succeed_symbol(),
                msg: succeed_message("Cancelled."),
            });
        }

        let spinner = Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            succeed_message("Logging you out"),
        );

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
        Err(e) => {
            // A 401 means the session is already expired on the server.
            // Treat this as success — the session is gone either way.
            if e.to_string().contains("Unauthorized") {
                Ok(())
            } else {
                Err(anyhow!("{e}"))
            }
        }
    }
}
