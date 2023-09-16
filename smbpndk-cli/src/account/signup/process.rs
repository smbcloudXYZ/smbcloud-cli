use super::SignupMethod;
use crate::account::lib::authorize_github;
use anyhow::{anyhow, Result};
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use log::debug;
use reqwest::{Client, StatusCode};
use serde::Serialize;
use smbpndk_model::{
    signup::{SignupEmailParams, SignupResult, SignupUserEmail},
    CommandResult,
};
use smbpndk_networking::{constants::PATH_USERS, smb_base_url_builder, smb_token_file_path};
use smbpndk_utils::email_validation;
use spinners::Spinner;

pub async fn process_signup() -> Result<CommandResult> {
    // Check if token file exists
    if smb_token_file_path().is_some() {
        return Ok(CommandResult {
            spinner: Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Loading...").green().bold().to_string(),
            ),
            symbol: "✅".to_owned(),
            msg: "You are already logged in. Please logout first.".to_owned(),
        });
    }

    let signup_methods = vec![SignupMethod::Email, SignupMethod::GitHub];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&signup_methods)
        .default(0)
        .interact_on_opt(&Term::stderr())
        .map(|i| signup_methods[i.unwrap()])
        .unwrap();

    match selection {
        SignupMethod::Email => signup_with_email(None).await,
        SignupMethod::GitHub => signup_with_github().await,
    }
}

pub async fn signup_with_email(email: Option<String>) -> Result<CommandResult> {
    let email = if let Some(email) = email {
        email
    } else {
        Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Username")
            .validate_with(|email: &String| email_validation(email))
            .interact()
            .unwrap()
    };

    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Password")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.len() >= 6 {
                Ok(())
            } else {
                Err("Password must be at least 6 characters")
            }
        })
        .with_confirmation("Confirm password", "Passwords do not match")
        .interact()
        .unwrap();

    let spinner = Spinner::new(
        spinners::Spinners::BouncingBall,
        style("Signing up...").green().bold().to_string(),
    );

    let params = SignupEmailParams {
        user: SignupUserEmail { email, password },
    };

    match do_signup(&params).await {
        Ok(_) => Ok(CommandResult {
            spinner,
            symbol: style("✅".to_string()).for_stderr().green().to_string(),
            msg: "You are signed up! Check your email to confirm your account.".to_owned(),
        }),
        Err(e) => Ok(CommandResult {
            spinner,
            symbol: style("✘".to_string()).for_stderr().red().to_string(),
            msg: format!("{e}"),
        }),
    }
}

async fn signup_with_github() -> Result<CommandResult> {
    match authorize_github().await {
        Ok(code) => {
            debug!("Code: {:#?}", code);
            Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::BouncingBall,
                    style("Requesting GitHub token...")
                        .green()
                        .bold()
                        .to_string(),
                ),
                symbol: style("✅".to_string()).for_stderr().green().to_string(),
                msg: "Finished requesting GitHub token!".to_owned(),
            })
        }
        Err(e) => {
            let error = anyhow!("Failed to get code from channel: {e}");
            Err(error)
        }
    }
}

pub async fn do_signup<T: Serialize + ?Sized>(args: &T) -> Result<CommandResult> {
    let spinner = Spinner::new(
        spinners::Spinners::BouncingBall,
        style("Signing you up...").green().bold().to_string(),
    );

    let response = Client::new()
        .post(build_smb_signup_url())
        .json(&args)
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => Ok(CommandResult {
            spinner,
            symbol: "✅".to_owned(),
            msg: "Your account has been created. Check email for verification link.".to_owned(),
        }),
        StatusCode::UNPROCESSABLE_ENTITY => {
            let result: SignupResult = response.json().await?;
            let error = anyhow!("Failed to signup: {}", result.status.message);
            Err(error)
        }
        _ => {
            let error = anyhow!("Failed to signup: {}", response.status());
            Err(error)
        }
    }
}

fn build_smb_signup_url() -> String {
    let mut url_builder = smb_base_url_builder();
    url_builder.add_route(PATH_USERS);
    url_builder.build()
}
