use {
    anyhow::{anyhow, Result},
    clap::Parser,
    console::style,
    smbcloud_cli::{
        account::{login::process_login, logout::process_logout, me::process_me, process_account},
        cli::{Cli, CommandResult, Commands},
        deploy::process_deploy::process_deploy,
        project::{crud_create::process_project_init, process::process_project},
    },
    smbcloud_network::{environment::Environment, network::check_internet_connection},
    std::{
        fs::{create_dir_all, OpenOptions},
        path::PathBuf,
        str::FromStr,
    },
    tracing::subscriber::set_global_default,
    tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer},
    tracing_subscriber::{filter::LevelFilter, prelude::*, EnvFilter},
};

fn setup_logging(env: Environment, level: Option<EnvFilter>) -> Result<()> {
    // Log in the current directory
    let log_path = match home::home_dir() {
        Some(path) => {
            create_dir_all(path.join(env.smb_dir()))?;
            let log_path = [
                path.to_str().unwrap(),
                "/",
                &env.smb_dir(),
                "/smbcloud-cli.log",
            ]
            .join("");
            // Create the file if it doesn't exist
            let _file = OpenOptions::new()
                .create(true)
                .truncate(true)
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
        .truncate(true)
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
            result.stop_and_persist();
            std::process::exit(0);
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

    // println!("Environment: {}", cli.environment);

    let log_level_error: Result<CommandResult> = Err(anyhow!(
        "Invalid log level: {:?}.\n Valid levels are: trace, debug, info, warn, and error.",
        cli.log_level
    ));

    if let Some(user_filter) = cli.log_level {
        let filter = match EnvFilter::from_str(&user_filter) {
            Ok(filter) => filter,
            Err(_) => return log_level_error,
        };
        setup_logging(cli.environment, Some(filter))?;
    } else {
        setup_logging(cli.environment, None)?;
    }

    // Check if the command requires internet connection
    let needs_internet = match &cli.command {
        Some(Commands::Me {})
        | Some(Commands::Deploy {})
        | Some(Commands::Login {})
        | Some(Commands::Logout {})
        | Some(Commands::Account { .. })
        | Some(Commands::Project { .. })
        | None => true,
        Some(Commands::Init {}) => true,
    };

    // Check internet connectivity for commands that need it
    if needs_internet && !check_internet_connection().await {
        return Err(anyhow!(
            "No internet connection. Please check your network settings and try again."
        ));
    }

    match cli.command {
        Some(Commands::Me {}) => process_me(cli.environment).await,
        Some(Commands::Init {}) => process_project_init(cli.environment).await,
        Some(Commands::Deploy {}) => process_deploy(cli.environment).await,
        Some(Commands::Account { command }) => process_account(cli.environment, command).await,
        Some(Commands::Login {}) => process_login(cli.environment).await,
        Some(Commands::Logout {}) => process_logout(cli.environment).await,
        Some(Commands::Project { command }) => process_project(cli.environment, command).await,
        None => process_deploy(cli.environment).await,
    }
}
