use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "List your mail apps.")]
    List {
        #[clap(long)]
        project_id: Option<String>,
    },
    #[clap(about = "Show a mail app.")]
    Show {
        #[clap(long, required = true)]
        id: String,
    },
    #[clap(about = "Create a mail app.")]
    New {
        #[clap(long, required = true)]
        name: String,
        #[clap(long, required = true)]
        domain: String,
        #[clap(long)]
        project_id: Option<String>,
        #[clap(long)]
        aws_region: Option<String>,
    },
    #[clap(about = "Update a mail app.")]
    Update {
        #[clap(long, required = true)]
        id: String,
        #[clap(long)]
        name: Option<String>,
        #[clap(long)]
        domain: Option<String>,
        #[clap(long)]
        aws_region: Option<String>,
    },
    #[clap(about = "Delete a mail app.")]
    Delete {
        #[clap(long, required = true)]
        id: String,
    },
    #[clap(about = "Manage mail inbox routes.")]
    Inbox {
        #[clap(subcommand)]
        command: InboxCommands,
    },
    #[clap(about = "Inspect inbound mail messages.")]
    Message {
        #[clap(subcommand)]
        command: MessageCommands,
    },
}

#[derive(Subcommand)]
pub enum InboxCommands {
    #[clap(about = "Create a mail inbox route.")]
    New {
        #[clap(long, required = true)]
        app_id: String,
        #[clap(long, required = true)]
        local_part: String,
        #[clap(long, required = true)]
        forward_to: String,
        #[clap(long)]
        sender_email: Option<String>,
    },
    #[clap(about = "Update a mail inbox route.")]
    Update {
        #[clap(long, required = true)]
        app_id: String,
        #[clap(long, required = true)]
        id: String,
        #[clap(long)]
        local_part: Option<String>,
        #[clap(long)]
        forward_to: Option<String>,
        #[clap(long)]
        sender_email: Option<String>,
    },
    #[clap(about = "Delete a mail inbox route.")]
    Delete {
        #[clap(long, required = true)]
        app_id: String,
        #[clap(long, required = true)]
        id: String,
    },
    #[clap(about = "Send a test email for a mail inbox route.")]
    Test {
        #[clap(long, required = true)]
        app_id: String,
        #[clap(long, required = true)]
        id: String,
        #[clap(long)]
        recipient_email: Option<String>,
        #[clap(long)]
        subject: Option<String>,
        #[clap(long)]
        body: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MessageCommands {
    #[clap(about = "List inbound mail messages for an inbox.")]
    List {
        #[clap(long, required = true)]
        app_id: String,
        #[clap(long, required = true)]
        inbox_id: String,
        #[clap(long, default_value_t = 10)]
        limit: u32,
    },
    #[clap(about = "Show one inbound mail message.")]
    Show {
        #[clap(long, required = true)]
        app_id: String,
        #[clap(long, required = true)]
        inbox_id: String,
        #[clap(long, required = true)]
        id: String,
    },
}
