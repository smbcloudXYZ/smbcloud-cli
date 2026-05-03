use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Create an account with your email address.")]
    Signup {},
    #[clap(about = "Log in to your account. If you need one first, run `smb signup`.")]
    Login {},
    #[clap(about = "Log out of your current session.")]
    Logout {},
    #[clap(about = "Start the password reset flow.")]
    Forgot {},
}
