use crate::{account, project};
use clap::{Parser, Subcommand};
use spinners::Spinner;

pub struct CommandResult {
    pub spinner: Spinner,
    pub symbol: String,
    pub msg: String,
}

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
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
    /*
    #[clap(about = "Manage your Oten authentication app. Add, delete, edit. Need authentication.")]
    Oten {
        #[clap(subcommand)]
        command: app_oten::cli::Commands,
    },
     */
    // Function
    /*
    FunApp {
        #[clap(subcommand)]
        command: fun_app::cli::Commands,
    }, */
    // Package
    /*
    PktApp {
        #[clap(subcommand)]
        command: pkt_app::cli::Commands,
    }, */
    // Relational database
    /*
    RdbApp {
        #[clap(subcommand)]
        command: rdb_app::cli::Commands,
    }, */
}
