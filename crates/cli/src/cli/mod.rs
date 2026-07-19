use {
    crate::{account, mail, project},
    clap::{Parser, Subcommand},
    smbcloud_network::environment::Environment,
    spinners::Spinner,
};

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

    /// Non-interactive mode for CI/automation: disable prompts. Confirmations
    /// use their default; prompts that need real input fail fast instead of
    /// blocking. Also enabled by SMB_CI=1 or the conventional CI env var.
    #[arg(long, global = true, env = "SMB_CI")]
    pub ci: bool,

    /// Full-screen TUI mode: render read commands in an interactive ratatui
    /// view instead of plain text. Mutually exclusive with --mcp.
    #[arg(long, global = true, conflicts_with = "mcp")]
    pub tui: bool,

    /// Run as an MCP (Model Context Protocol) server over stdio instead of a
    /// one-shot command. Implies non-interactive; the subcommand is ignored.
    #[arg(long, global = true)]
    pub mcp: bool,

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
    Deploy {
        /// Name of the sub-project to deploy (for monorepo configs with [[projects]]).
        /// Matches the `name` field in .smb/config.toml. Omit to deploy the root project.
        #[arg(short, long)]
        project: Option<String>,
    },
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
    #[clap(about = "Manage smbCloud Mail.")]
    Mail {
        #[clap(subcommand)]
        command: mail::cli::Commands,
    },
    #[clap(
        about = "Migrate local .smb/config.toml deploy fields to the smbCloud server.",
        display_order = 4
    )]
    Migrate {},
}
