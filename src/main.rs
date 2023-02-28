use std::{fs::OpenOptions, path::PathBuf, str::FromStr, time::Duration};

use anyhow::{anyhow, Result};
use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Password};
use smbpndk_cli::{
    cli::{Cli, Commands},
    constants::ERROR_EMOJI,
    login::{process_login, LoginArgs},
    signup::{process_signup, SignupArgs},
};
use spinners::Spinner;
use tokio::time::sleep;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{filter::LevelFilter, prelude::*, EnvFilter};

fn setup_logging(level: Option<EnvFilter>) -> Result<()> {
    // Log in the current directory
    let log_path = PathBuf::from("smbpndk-cli.log");

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
        }
        Err(e) => {
            println!("\n{} {}", ERROR_EMOJI, style(e).red());
            std::process::exit(1);
        }
    }
}

struct CommandResult {
    spinner: Spinner,
    symbol: String,
    msg: String,
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
        Commands::Login {} => {
            println!("Provide your login credentials.");
            let username = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Username")
                .interact()
                .unwrap();
            let password = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Password")
                .interact()
                .unwrap();

            let spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Logging in...").green().bold().to_string(),
            );

            match process_login(LoginArgs { username, password }).await {
                Ok(_) => {
                    return Ok(CommandResult {
                        spinner: spinner,
                        symbol: "✅".to_owned(),
                        msg: "You are logged in!".to_owned(),
                    });
                }
                Err(e) => {
                    return Ok(CommandResult {
                        spinner: spinner,
                        symbol: "❌".to_owned(),
                        msg: format!("Failed to login: {}", e),
                    });
                }
            }
        }
        Commands::Signup { username, password } => {
            let spinner = Spinner::new(
                spinners::Spinners::BouncingBall,
                style("Signing up...").green().bold().to_string(),
            );
            let join_handle = tokio::spawn(async move {
                let _future = process_signup(SignupArgs { username, password }).await;
                sleep(Duration::from_millis(5000)).await;
            });

            match join_handle.await {
                Ok(_) => {
                    return Ok(CommandResult {
                        spinner: spinner,
                        symbol: "✅".to_owned(),
                        msg: "You are signed up!".to_owned(),
                    });
                }
                Err(e) => {
                    return Ok(CommandResult {
                        spinner: spinner,
                        symbol: "❌".to_owned(),
                        msg: format!("Failed to signup: {}", e),
                    });
                }
            }
        }
    }
}
