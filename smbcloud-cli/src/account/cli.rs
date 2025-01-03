use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Create an account. Use your email as your username.")]
    Signup {},
    #[clap(about = "Login to your account. To create an account, use smb signup.")]
    Login {},
    #[clap(about = "Logout all session.")]
    Logout {},
    #[clap(about = "Forgot email? Use this command to reset your password.")]
    Forgot {},
}
