use {
    crate::{account, cloud_auth, mail, project, tenant},
    clap::{Parser, Subcommand, ValueEnum},
    smbcloud_network::environment::Environment,
    spinners::Spinner,
    std::path::PathBuf,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum McpScope {
    #[default]
    Cloud,
    Automation,
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

    /// MCP tool profile to expose. Cloud serves smbCloud resources; automation
    /// serves cross-platform mobile and TV device tools.
    #[arg(
        long = "scope",
        global = true,
        value_enum,
        default_value_t,
        requires = "mcp"
    )]
    pub mcp_scope: McpScope,

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
    #[clap(about = "Manage your tenants.")]
    Tenant {
        #[clap(subcommand)]
        command: tenant::cli::Commands,
    },
    #[clap(about = "Manage smbCloud Mail.")]
    Mail {
        #[clap(subcommand)]
        command: mail::cli::Commands,
    },
    #[clap(about = "Manage smbCloud Auth apps.")]
    Auth {
        #[clap(subcommand)]
        command: cloud_auth::cli::Commands,
    },
    #[clap(about = "Run and control XCRS ControlKit on Apple devices.")]
    ControlKit {
        #[clap(subcommand)]
        command: ControlKitCommands,
    },
    #[clap(
        about = "Migrate local .smb/config.toml deploy fields to the smbCloud server.",
        display_order = 4
    )]
    Migrate {},
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_defaults_to_cloud_scope() {
        let cli = Cli::try_parse_from(["smb", "--mcp"]).expect("MCP arguments should parse");

        assert_eq!(cli.mcp_scope, McpScope::Cloud);
    }

    #[test]
    fn mcp_accepts_automation_scope() {
        let cli = Cli::try_parse_from(["smb", "--mcp", "--scope", "automation"])
            .expect("automation MCP arguments should parse");

        assert_eq!(cli.mcp_scope, McpScope::Automation);
    }

    #[test]
    fn scope_requires_mcp_mode() {
        // `Cli` has no `Debug` impl (clap's `Parser` doesn't require one), so
        // `expect_err`/`unwrap_err` (which bound the `Ok` type on `Debug`)
        // don't apply here; match the `Result` instead.
        let result = Cli::try_parse_from(["smb", "--scope", "automation"]);
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("scope without MCP mode should be rejected"),
        };

        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }
}

#[derive(Subcommand)]
pub enum ControlKitCommands {
    #[clap(about = "Build a signed ControlKit XCTest runner for a physical device.")]
    Build {
        #[arg(long)]
        project_path: PathBuf,
        #[arg(long, default_value = "ControlKit")]
        scheme: String,
        #[arg(long, default_value = "Release")]
        configuration: String,
        #[arg(long)]
        device_udid: String,
        #[arg(long)]
        derived_data_path: PathBuf,
    },
    #[clap(about = "Start a built ControlKit XCTest runner on a physical device.")]
    Start {
        #[arg(long)]
        device_udid: String,
        #[arg(long)]
        xctestrun_path: PathBuf,
        #[arg(long, default_value = "::")]
        listen_host: String,
        #[arg(long, default_value_t = 12004)]
        listen_port: u16,
        #[arg(long, default_value_t = 120)]
        timeout_seconds: u64,
        #[arg(long)]
        log_path: Option<PathBuf>,
    },
    #[clap(about = "Call a running ControlKit JSON-RPC method.")]
    Call {
        #[arg(long, required_unless_present = "host", conflicts_with = "host")]
        device_udid: Option<String>,
        #[arg(long, conflicts_with = "device_udid")]
        host: Option<String>,
        #[arg(long, default_value_t = 12004)]
        port: u16,
        #[arg(long)]
        method: String,
        #[arg(long, default_value = "{}")]
        params: String,
    },
}
