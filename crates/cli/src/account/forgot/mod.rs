use crate::{
    cli::CommandResult,
    ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
};
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input, Password};
use reqwest::{Client, StatusCode};
use smbcloud_model::forgot::{Args, Email, Param, UserUpdatePassword};
use smbcloud_network::environment::Environment;
use smbcloud_networking::{constants::PATH_USERS_PASSWORD, smb_base_url_builder};
use smbcloud_utils::email_validation;
use spinners::Spinner;

pub async fn process_forgot(env: Environment) -> Result<CommandResult> {
    println!("Provide your login credentials.");
    let email = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Email")
        .validate_with(|email: &String| email_validation(email))
        .interact()
        .unwrap();
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Checking email"),
    );

    let params = Args {
        user: Email { email },
    };

    let response = Client::new()
        .post(build_smb_forgot_url(env))
        .json(&params)
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("Check your email and input your code here."),
            );
            input_code(env).await
        }
        _ => Ok(CommandResult {
            spinner,
            symbol: fail_symbol(),
            msg: fail_message("Something wrong when trying to reset email."),
        }),
    }
}

async fn input_code(env: Environment) -> Result<CommandResult> {
    let security_code = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Code")
        .interact()
        .unwrap();

    Spinner::new(
        spinners::Spinners::Hamburger,
        succeed_message("Checking your code."),
    )
    .stop_and_persist("✅", "Great. Now input your new password.".to_owned());

    let new_password = Password::with_theme(&ColorfulTheme::default())
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
    let password_confirmation = String::from(&new_password);

    // Should reuse this somehow
    let params = Param {
        user: UserUpdatePassword {
            reset_password_token: security_code,
            password: new_password,
            password_confirmation,
        },
    };

    let spinner = Spinner::new(
        spinners::Spinners::Hamburger,
        succeed_message("Updating your password."),
    );

    let response = Client::new()
        .put(build_smb_forgot_url(env))
        .json(&params)
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Your password has been updated. Login with your new password."),
        }),
        StatusCode::NOT_FOUND => Ok(CommandResult {
            spinner,
            symbol: fail_symbol(),
            msg: fail_message("URL not found."),
        }),
        _ => Ok(CommandResult {
            spinner,
            symbol: fail_symbol(),
            msg: fail_message("Something wrong when trying to reset email."),
        }),
    }
}

fn build_smb_forgot_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_PASSWORD);
    url_builder.build()
}
