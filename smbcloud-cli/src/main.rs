use anyhow::{anyhow, Result};
use clap::Parser;
use console::style;
use smbcloud_cli::cli::CommandResult;
use smbcloud_cli::{
    account::process_account,
    cli::{Cli, Commands},
    project::process_project,
    deploy::process_deploy,
};
use std::{
    fs::{create_dir_all, OpenOptions},
    path::PathBuf,
    str::FromStr,
};
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{filter::LevelFilter, prelude::*, EnvFilter};

fn setup_logging(level: Option<EnvFilter>) -> Result<()> {
    // Log in the current directory
    let log_path = match home::home_dir() {
        Some(path) => {
            create_dir_all(path.join(".smb"))?;
            let log_path = [path.to_str().unwrap(), "/.smb/smbcloud-cli.log"].join("");
            // Create the file if it doesn't exist
            let _file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&log_path)?;

            PathBuf::from(log_path)
        }
        None => {
            return Err(anyhow!("Could not find home directory."));
        }
    };

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(log_path)
        .unwrap();

    let env_filter = if let Some(filter) = level {
        filter
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace"))
    };

    let formatting_layer = BunyanFormattingLayer::new("smb".into(), file);
    let level_filter = LevelFilter::from_str(&env_filter.to_string())?;

    let subscriber = tracing_subscriber::registry()
        .with(formatting_layer.with_filter(level_filter))
        .with(JsonStorageLayer);

    set_global_default(subscriber).expect("Failed to set global default subscriber");

    Ok(())
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(result) => {
            let mut spinner = result.spinner;
            spinner.stop_and_persist(&result.symbol, result.msg);
            std::process::exit(1);
        }
        Err(e) => {
            println!(
                "\n{} {}",
                style("✘".to_string()).for_stderr().red(),
                style(e).red()
            );
            std::process::exit(1);
        }
    }
}

async fn run() -> Result<CommandResult> {
    let cli = Cli::parse();

    let log_level_error: Result<CommandResult> = Err(anyhow!(
        "Invalid log level: {:?}.\n Valid levels are: trace, debug, info, warn, and error.",
        cli.log_level
    ));

    if let Some(user_filter) = cli.log_level {
        let filter = match EnvFilter::from_str(&user_filter) {
            Ok(filter) => filter,
            Err(_) => return log_level_error,
        };
        setup_logging(Some(filter))?;
    } else {
        setup_logging(None)?;
    }

    match cli.command {
        Commands::Account { command } => process_account(command).await,
        Commands::Project { command } => process_project(command).await,
        Commands::Deploy {  } => process_deploy().await,
    }
}
