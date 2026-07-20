use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "List your Auth apps.")]
    List {
        #[clap(long)]
        project_id: Option<String>,
    },
    #[clap(about = "Show an Auth app.")]
    Show {
        #[clap(long, required = true)]
        id: String,
    },
    #[clap(about = "Create an Auth app.")]
    New {
        #[clap(long, required = true)]
        name: String,
        #[clap(long)]
        project_id: Option<String>,
        #[clap(long)]
        support_email: Option<String>,
    },
    #[clap(about = "Update an Auth app.")]
    Update {
        #[clap(long, required = true)]
        id: String,
        #[clap(long)]
        name: Option<String>,
        #[clap(long)]
        support_email: Option<String>,
    },
    #[clap(about = "Delete an Auth app.")]
    Delete {
        #[clap(long, required = true)]
        id: String,
    },
}
