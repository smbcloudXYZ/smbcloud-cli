use crate::{account, project};
use clap::{Parser, Subcommand};
use network::environment::Environment;
use spinners::Spinner;

pub struct CommandResult {
    pub spinner: Spinner,
    pub symbol: String,
    pub msg: String,
}

impl CommandResult {
    pub fn stop_and_persist(mut self) {
        self.spinner.stop_and_persist(&self.symbol, self.msg);
    }
}

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Environment: dev, production
    #[arg(short, long, env = "ENVIRONMENT", default_value = "production")]
    pub environment: Environment,

    /// Log level: trace, debug, info, warn, error, off
    #[clap(short, long, global = true)]
    pub log_level: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Your account info.", display_order = 3)]
    Me {},
    #[clap(
        about = "Deploy project. This is smb main command. Requires an smbCloud account.",
        display_order = 0
    )]
    Deploy {},
    #[clap(
        about = "Initialize project. Requires an smbCloud account.",
        display_order = 1
    )]
    Init {},
    #[clap(about = "Login to your account.", display_order = 2)]
    Login {},
    #[clap(about = "Logout from your account.", display_order = 3)]
    Logout {},
    #[clap(about = "Manage your account.")]
    Account {
        #[clap(subcommand)]
        command: account::cli::Commands,
    },
    #[clap(about = "Manage your projects.")]
    Project {
        #[clap(subcommand)]
        command: project::cli::Commands,
    },
}
