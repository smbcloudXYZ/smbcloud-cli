pub mod cli;
pub mod forgot;
pub mod lib;
pub mod login;
pub mod signup;

use self::{
    cli::Commands,
    forgot::process_forgot,
    login::{process_login, process_logout},
    signup::process_signup,
};
use crate::cli::CommandResult;
use anyhow::Result;

pub async fn process_account(commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::Signup {} => process_signup().await,
        Commands::Login {} => process_login().await,
        Commands::Logout {} => process_logout().await,
        Commands::Forgot {} => process_forgot().await,
    }
}
