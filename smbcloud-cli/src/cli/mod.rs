use crate::{account, project};
use clap::{Parser, Subcommand};
use smbcloud_networking::environment::Environment;
use spinners::Spinner;

pub struct CommandResult {
    pub spinner: Spinner,
    pub symbol: String,
    pub msg: String,
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

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Manage your account.")]
    Account {
        #[clap(subcommand)]
        command: account::cli::Commands,
    },

    #[clap(about = "Manage your projects. Add, delete, edit. Need authentication.")]
    Project {
        #[clap(subcommand)]
        command: project::cli::Commands,
    },
    #[clap(about = "Initialize project. Requires an smbCloud account.")]
    Init {
        /// Project name
        #[clap(short, long, required = false)]
        name: Option<String>,
        /// Project description
        #[clap(short, long, required = false)]
        description: Option<String>,
    },
    #[clap(about = "Deploy project. It will use deploy.sh script in the .smb folder.")]
    Deploy {},
}
