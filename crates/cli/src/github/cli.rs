use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Install the smbCloud GitHub App on your account or organization.")]
    Install {},

    #[clap(about = "Connect a GitHub repository to a project for auto-deploy on push.")]
    Connect {
        /// GitHub repository as owner/name. Omit to pick interactively.
        #[clap(long)]
        repo: Option<String>,

        /// Branch that triggers production deploys. Defaults to the
        /// repository's default branch.
        #[clap(long)]
        branch: Option<String>,

        /// Project to connect. Defaults to the current project (`smb project use`).
        #[clap(long)]
        project_id: Option<String>,
    },

    #[clap(about = "Show the GitHub connection for a project.")]
    Status {
        /// Project to inspect. Defaults to the current project.
        #[clap(long)]
        project_id: Option<String>,
    },

    #[clap(about = "Disconnect the GitHub repository from a project.")]
    Disconnect {
        /// Project to disconnect. Defaults to the current project.
        #[clap(long)]
        project_id: Option<String>,
    },
}
