use smbcloud_networking::smb_client::SmbClient;

pub mod account;
pub mod ci;
pub mod cli;
#[path = "cloud-auth/mod.rs"]
pub mod cloud_auth;
#[path = "cloud-deploy/mod.rs"]
pub mod deploy;
pub mod interface;
#[path = "cloud-mail/mod.rs"]
pub mod mail;
pub mod mcp;
pub mod project;
mod token;
pub mod ui;

pub use token::clear_smb_token::clear_smb_token;

pub(crate) fn client() -> (&'static SmbClient, &'static str) {
    let secret = env!("CLI_CLIENT_SECRET");
    (&SmbClient::Cli, secret)
}
