use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Add new project.")]
    New {
        /// No init project by default. If this flag exist, we'll init the project.
        #[clap(short, long, required = false)]
        init: bool,
    },

    #[clap(about = "List all your projects.")]
    List {},

    #[clap(about = "Show detail of a project.")]
    Show {
        /// Project Id
        #[clap(short, long, required = true)]
        id: String,
    },

    #[clap(about = "Delete a project.")]
    Delete {
        /// Project name
        #[clap(short, long, required = true)]
        id: String,
    },

    #[clap(about = "Use project for current CLI session.")]
    Use {
        #[clap(short, long, required = true)]
        id: String,
    },
    #[clap(about = "Manage project deployment. Pass --id to get detail deployment.")]
    Deployment {
        #[clap(short, long)]
        id: Option<String>,
    },
    #[clap(about = "Update project description. Specify `--id` to update specific project.")]
    Update {
        #[clap(short, long)]
        id: String,
    },
}
