//! Admin view of the Run fleet. Both commands need an admin user's login; the
//! API refuses anyone else.

use crate::{
    cli::CommandResult,
    client,
    token::get_smb_token::get_smb_token,
    ui::{succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use clap::Subcommand;
use console::style;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use smbcloud_model::error_codes::ErrorResponse;
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_base_url_builder, smb_client::SmbClient};
use spinners::Spinner;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Manage Run runners (admin only).")]
    Runners {
        #[clap(subcommand)]
        command: RunnerCommands,
    },
}

#[derive(Subcommand)]
pub enum RunnerCommands {
    #[clap(about = "List runners with their status and tags.")]
    List {},
    #[clap(about = "Replace a runner's tags. Executor tags the runner reports are kept.")]
    Tags {
        /// The runner ID.
        id: i64,
        /// The full tag list. Lower-case letters, digits and hyphens, up to 16.
        #[clap(required = true)]
        tags: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub tags: Vec<String>,
    pub ci_enabled: bool,
    pub publish_enabled: bool,
    pub last_heartbeat_at: Option<String>,
    pub version: Option<String>,
    pub hostname: Option<String>,
    pub active_jobs: i64,
}

#[derive(Serialize)]
struct TagsBody {
    tags: Vec<String>,
}

fn runners_url(env: Environment, client: (&SmbClient, &str), id: Option<i64>) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/admin/run/runners");
    if let Some(id) = id {
        url_builder.add_route(&id.to_string());
    }
    url_builder.build()
}

pub(crate) async fn list_runners(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
) -> Result<Vec<Runner>, ErrorResponse> {
    let builder = Client::new()
        .get(runners_url(env, client, None))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub(crate) async fn set_runner_tags(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    id: i64,
    tags: Vec<String>,
) -> Result<Runner, ErrorResponse> {
    let builder = Client::new()
        .patch(runners_url(env, client, Some(id)))
        .json(&TagsBody { tags })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn process_run(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::Runners { command } => match command {
            RunnerCommands::List {} => process_runners_list(env).await,
            RunnerCommands::Tags { id, tags } => process_runner_tags(env, id, tags).await,
        },
    }
}

async fn process_runners_list(env: Environment) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mut spinner = loading_spinner("Loading runners");

    let runners = list_runners(env, client(), access_token)
        .await
        .map_err(|error| anyhow!(error.to_string()))?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    println!("\n{}", style("Runners").bold().underlined());
    for runner in &runners {
        print_runner(runner);
    }

    Ok(done_result(if runners.is_empty() {
        "No runners found."
    } else {
        "Done."
    }))
}

async fn process_runner_tags(
    env: Environment,
    id: i64,
    tags: Vec<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mut spinner = loading_spinner("Saving tags");

    let runner = set_runner_tags(env, client(), access_token, id, tags)
        .await
        .map_err(|error| anyhow!(error.to_string()))?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Saved."));
    print_runner(&runner);

    Ok(done_result("Tags updated."))
}

fn print_runner(runner: &Runner) {
    println!(
        "#{} {} [{}] tags={} active_jobs={} host={}",
        runner.id,
        style(&runner.name).bold(),
        runner.status,
        runner.tags.join(","),
        runner.active_jobs,
        runner.hostname.as_deref().unwrap_or("-"),
    );
}

fn loading_spinner(message: &str) -> Spinner {
    Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message(message),
    )
}

fn done_result(message: &str) -> CommandResult {
    CommandResult {
        spinner: loading_spinner("Done"),
        symbol: succeed_symbol(),
        msg: succeed_message(message),
    }
}
