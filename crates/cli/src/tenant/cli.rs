use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "List your tenants.")]
    List {},
    #[clap(about = "Show a tenant.")]
    Show {
        #[clap(short, long, required = true)]
        id: String,
    },
    #[clap(about = "Create an organization tenant.")]
    New {
        #[clap(short, long, required = true)]
        name: String,
    },
    #[clap(about = "Rename a tenant.")]
    Update {
        #[clap(short, long, required = true)]
        id: String,
        #[clap(short, long, required = true)]
        name: String,
    },
    #[clap(about = "Delete an organization tenant.")]
    Delete {
        #[clap(short, long, required = true)]
        id: String,
    },
}
