pub mod cli;
pub mod forgot;
pub mod lib;
pub mod login;
pub mod signup;
pub mod logout;

use self::{
    cli::Commands,
    forgot::process_forgot,
    login::process_login,
    signup::process_signup,
};
use crate::cli::CommandResult;
use anyhow::Result;
use logout::process_logout;
use smbcloud_networking::environment::Environment;

pub async fn process_account(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::Signup {} => process_signup(env).await,
        Commands::Login {} => process_login(env).await,
        Commands::Logout {} => process_logout(env).await,
        Commands::Forgot {} => process_forgot(env).await,
    }
}
