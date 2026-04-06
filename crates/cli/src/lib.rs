use smbcloud_networking::smb_client::SmbClient;

pub mod account;
pub mod cli;
pub mod deploy;
pub mod project;
mod token;
mod ui;

pub(crate) fn client() -> (&'static SmbClient, &'static str) {
    let secret = env!("CLI_CLIENT_SECRET");
    (&SmbClient::Cli, secret)
}
