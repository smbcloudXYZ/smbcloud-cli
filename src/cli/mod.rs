use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Create a new project")]
    Login {
        #[clap(short, long)]
        username: String,
        #[clap(short, long)]
        password: String,
    },
}