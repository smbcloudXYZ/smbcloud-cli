use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Add new project.")]
    New {},

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
    }
}
