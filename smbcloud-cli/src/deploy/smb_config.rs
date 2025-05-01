use console::style;
use smbcloud_networking::{environment::Environment, get_smb_token};
use spinners::Spinner;
use anyhow::Result;
use crate::cli::CommandResult;

pub(crate) async fn check_config(env: Environment) -> Result<CommandResult> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Checking config...").green().bold().to_string(),
    );

    let token = match get_smb_token(env).await {
        Ok(token) => token,
        Err(e) => {
            spinner.stop_and_persist("😩", e.to_string());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "Failed to get SMB token.".to_owned(),
            });
        }
    };

    Ok(CommandResult {
        spinner,
        symbol: "✅".to_owned(),
        msg: format!("SMB Token: {}", token),
    })
}