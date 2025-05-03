use std::fs;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm};
use reqwest::{Client, StatusCode};
use smbcloud_networking::{constants::PATH_USERS_SIGN_OUT, environment::Environment, get_smb_token, smb_base_url_builder, smb_token_file_path};
use spinners::Spinner;
use anyhow::{anyhow, Result};

use crate::{cli::CommandResult, ui::{fail_message, fail_symbol, succeed_message, succeed_symbol}};

pub async fn process_logout(env: Environment) -> Result<CommandResult> {
  // Logout if user confirms
  if let Some(token_path) = smb_token_file_path(env) {
      let confirm = match Confirm::with_theme(&ColorfulTheme::default())
          .with_prompt("Do you want to logout? y/n")
          .interact()
      {
          Ok(confirm) => confirm,
          Err(_) => {
              let error = anyhow!("Invalid input.");
              return Err(error);
          }
      };
      if !confirm {
          return Ok(CommandResult {
              spinner: Spinner::new(
                  spinners::Spinners::SimpleDotsScrolling,
                  style("Cancel operation.").green().bold().to_string(),
              ),
              symbol: succeed_symbol(),
              msg: "Doing nothing.".to_owned(),
          });
      }

      let spinner = Spinner::new(
          spinners::Spinners::SimpleDotsScrolling,
          style("Logging you out...").green().bold().to_string(),
      );

      // Call backend
      match do_process_logout(env).await {
          Ok(_) => {
              fs::remove_file(token_path)?;
              Ok(CommandResult {
                  spinner,
                  symbol: succeed_symbol(),
                  msg: succeed_message("You are logged out!"),
              })
          }
          Err(e) => Err(anyhow!("{e}")),
      }
  } else {
      Ok(CommandResult {
          spinner: Spinner::new(
              spinners::Spinners::SimpleDotsScrolling,
              style("Loading...").green().bold().to_string(),
          ),
          symbol: fail_symbol(),
          msg: fail_message("You are not logged in. Please login first."),
      })
  }
}

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

fn build_smb_logout_url(env: Environment) -> String {
  let mut url_builder = smb_base_url_builder(env);
  url_builder.add_route(PATH_USERS_SIGN_OUT);
  url_builder.build()
}
