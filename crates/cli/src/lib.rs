use smbcloud_networking::smb_client::SmbClient;

pub mod account;
pub mod ci;
pub mod cli;
pub mod deploy;
pub mod github;
pub mod mail;
pub mod project;
mod token;
pub mod ui;

pub use token::clear_smb_token::clear_smb_token;

pub(crate) fn client() -> (&'static SmbClient, &'static str) {
    let secret = env!("CLI_CLIENT_SECRET");
    (&SmbClient::Cli, secret)
}
