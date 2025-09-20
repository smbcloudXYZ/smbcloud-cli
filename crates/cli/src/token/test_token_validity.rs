use crate::token::{get_smb_token, smb_token_file_path};
use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use reqwest::{Client, StatusCode};
use smbcloud_network::environment::Environment;
use smbcloud_networking::{constants::PATH_USERS_SIGN_OUT, smb_base_url_builder};
use spinners::Spinner;
use std::fs;

async fn do_process_logout(env: Environment) -> Result<()> {
    let token = get_smb_token(env).await?;

    let response = Client::new()
        .delete(build_smb_logout_url(env))
        .header("Authorization", token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => Ok(()),
        _ => Err(anyhow!("Failed to logout.")),
    }
}
