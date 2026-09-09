use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use xcrs::{AndroidDebugBridge, AndroidKit, IosAppTest, XcodeCommandLineTools};

#[derive(Debug, Parser)]
#[command(name = "xcrs")]
#[command(version)]
#[command(about = "Cross-platform mobile and TV app automation")]
struct Cli {
    /// Run as a standalone Model Context Protocol server over stdio.
    #[arg(long)]
    mcp: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(subcommand)]
    Simulator(SimulatorCommand),
    #[command(subcommand)]
    AndroidKit(AndroidKitCommand),
    IosAppTest {
        #[arg(long, conflicts_with = "simulator_udid")]
        simulator_name: Option<String>,
        #[arg(long, conflicts_with = "simulator_name")]
        simulator_udid: Option<String>,
        #[arg(long)]
        app_path: PathBuf,
        #[arg(long)]
        bundle_id: String,
        #[arg(long)]
        terminate_before_launch: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum AndroidKitCommand {
    Install {
        #[arg(long)]
        serial: String,
        #[arg(long)]
        apk: PathBuf,
    },
    Start {
        #[arg(long)]
        serial: String,
    },
    Status {
        #[arg(long)]
        serial: String,
        #[arg(long)]
        json: bool,
    },
    Stop {
        #[arg(long)]
        serial: String,
    },
}

#[derive(Debug, Subcommand)]
enum SimulatorCommand {
    Find {
        #[arg(long)]
        name: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.mcp {
        return xcrs::mcp::serve().await;
    }

    let command = cli
        .command
        .ok_or_else(|| anyhow!("a command is required unless --mcp is provided"))?;
    let tools = XcodeCommandLineTools::new();

    match command {
        Commands::AndroidKit(AndroidKitCommand::Install { serial, apk }) => {
            AndroidDebugBridge::new().install_androidkit(&serial, &apk)?;
            println!("Installed XCRS AndroidKit on {serial}");
        }
        Commands::AndroidKit(AndroidKitCommand::Start { serial }) => {
            AndroidDebugBridge::new().start_androidkit(&serial)?;
            println!("Started XCRS AndroidKit on {serial}");
        }
        Commands::AndroidKit(AndroidKitCommand::Status { serial, json }) => {
            let info = AndroidKit::new()
                .call(serial.clone(), "device.info", serde_json::json!({}))
                .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("XCRS AndroidKit is ready on {serial}");
            }
        }
        Commands::AndroidKit(AndroidKitCommand::Stop { serial }) => {
            AndroidDebugBridge::new().stop_androidkit(&serial)?;
            println!("Stopped XCRS AndroidKit on {serial}");
        }
        Commands::Simulator(SimulatorCommand::Find { name, json }) => {
            let simulator = tools.simctl().find_simulator_by_name(&name)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&simulator)?);
            } else {
                println!("{} {}", simulator.name, simulator.udid);
            }
        }
        Commands::Simulator(SimulatorCommand::List { json }) => {
            let simulators = tools.simctl().list_simulators()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&simulators)?);
            } else {
                for simulator in simulators {
                    println!("{} {} {}", simulator.name, simulator.udid, simulator.state);
                }
            }
        }
        Commands::IosAppTest {
            simulator_name,
            simulator_udid,
            app_path,
            bundle_id,
            terminate_before_launch,
            json,
        } => {
            let result = tools.run_ios_app_test(&IosAppTest {
                simulator_name,
                simulator_udid,
                app_path,
                bundle_id,
                terminate_before_launch,
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!(
                    "Launched {} on {} ({})",
                    result.bundle_id, result.simulator.name, result.simulator.udid
                );
            }
        }
    }

    Ok(())
}
