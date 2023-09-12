use crate::account::{
    forgot::{Param, UserUpdatePassword},
    lib::{authorize_github, save_token, ErrorCode, GithubInfo, SmbAuthorization},
    model::{Data, Status, User},
    signup::{
        do_signup, GithubEmail, Provider, SignupGithubParams, SignupMethod, SignupUserGithub,
    },
};
use anyhow::{anyhow, Result};
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use log::debug;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use smbpndk_model::CommandResult;
use smbpndk_networking::{get_smb_token, smb_base_url_builder, smb_token_file_path};
use smbpndk_utils::email_validation;
use spinners::Spinner;
use std::fs::{self};

pub struct LoginArgs {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
struct LoginParams {
    user: UserParam,
}

#[derive(Debug, Serialize)]
struct UserParam {
    email: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResult {
    status: Status,
    data: Data,
}

pub async fn process_login() -> Result<CommandResult> {
    // Check if token file exists
    if smb_token_file_path().is_some() {
        return Ok(CommandResult {
            spinner: Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Loading...").green().bold().to_string(),
            ),
            symbol: "✅".to_owned(),
            msg: "You are already logged in.".to_owned(),
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
        SignupMethod::Email => login_with_email().await,
        SignupMethod::GitHub => login_with_github().await,
    }
}

pub async fn process_logout() -> Result<CommandResult> {
    // Logout if user confirms
    if let Some(token_path) = smb_token_file_path() {
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to logout? y/n")
            .interact()
            .unwrap();
        if !confirm {
            return Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    style("Cancel operation.").green().bold().to_string(),
                ),
                symbol: "✅".to_owned(),
                msg: "Doing nothing.".to_owned(),
            });
        }

        let mut spinner = Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            style("Logging you out...").green().bold().to_string(),
        );

        // Call backend
        match do_process_logout().await {
            Ok(_) => {
                spinner.stop_and_persist("✅", "Done.".to_owned());
                fs::remove_file(token_path)?;
                Ok(CommandResult {
                    spinner: Spinner::new(
                        spinners::Spinners::SimpleDotsScrolling,
                        style("Loading...").green().bold().to_string(),
                    ),
                    symbol: "✅".to_owned(),
                    msg: "You are now logged out!".to_owned(),
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
            symbol: "😏".to_owned(),
            msg: "You are not logged in.".to_owned(),
        })
    }
}

// Private functions

async fn login_with_github() -> Result<CommandResult> {
    match authorize_github().await {
        Ok(result) => process_authorization(result).await,
        Err(err) => {
            let error = anyhow!("Failed to authorize your GitHub account. {}", err);
            Err(error)
        }
    }
}

async fn process_authorization(auth: SmbAuthorization) -> Result<CommandResult> {
    // What to do if not logged in with GitHub?
    // Check error_code first
    if let Some(error_code) = auth.error_code {
        debug!("{}", error_code);
        match error_code {
            ErrorCode::EmailNotFound => {
                return create_new_account(auth.user_email, auth.user_info).await
            }
            ErrorCode::EmailUnverified => return send_email_verification(auth.user).await,
            ErrorCode::PasswordNotSet => {
                // Only for email and password login
                let error = anyhow!("Password not set.");
                return Err(error);
            }
            ErrorCode::GithubNotLinked => return connect_github_account(auth).await,
        }
    }

    // Logged in with GitHub!
    // Token handling is in the lib.rs account module.
    if let Some(user) = auth.user {
        let spinner = Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            style("Logging you in...").green().bold().to_string(),
        );
        // We're logged in with GitHub.
        return Ok(CommandResult {
            spinner,
            symbol: "✅".to_owned(),
            msg: format!("You are logged in with GitHub as {}.", user.email),
        });
    }

    let error: anyhow::Error = anyhow!("Failed to login with GitHub.");
    Err(error)
}

async fn create_new_account(
    user_email: Option<GithubEmail>,
    user_info: Option<GithubInfo>,
) -> Result<CommandResult> {
    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to create a new account?")
        .interact()
        .unwrap();

    // Create account if user confirms
    if !confirm {
        let spinner = Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            style("Logging you in...").green().bold().to_string(),
        );
        return Ok(CommandResult {
            spinner,
            symbol: "✅".to_owned(),
            msg: "Please accept to link your GitHub account.".to_owned(),
        });
    }

    if let (Some(email), Some(info)) = (user_email, user_info) {
        let params = SignupGithubParams {
            user: SignupUserGithub {
                email: email.email,
                authorizations_attributes: vec![Provider {
                    uid: info.id.to_string(),
                    provider: 0,
                }],
            },
        };

        return do_signup(&params).await;
    }

    Err(anyhow!("Shouldn't be here."))
}

async fn send_email_verification(user: Option<User>) -> Result<CommandResult> {
    // Return early if user is null
    if let Some(user) = user {
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to send a new verification email?")
            .interact()
            .unwrap();

        // Send verification email if user confirms
        if !confirm {
            let spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Cancel operation.").green().bold().to_string(),
            );
            return Ok(CommandResult {
                spinner,
                symbol: "✅".to_owned(),
                msg: "Doing nothing.".to_owned(),
            });
        }
        resend_email_verification(user).await
    } else {
        let error = anyhow!("Failed to get user.");
        Err(error)
    }
}

async fn resend_email_verification(user: User) -> Result<CommandResult> {
    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Sending verification email...")
            .green()
            .bold()
            .to_string(),
    );

    let response = Client::new()
        .post(build_smb_resend_email_verification_url())
        .body(format!("id={}", user.id))
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => Ok(CommandResult {
            spinner,
            symbol: "✅".to_owned(),
            msg: "Verification email sent!".to_owned(),
        }),
        _ => {
            let error = anyhow!("Failed to send verification email.");
            Err(error)
        }
    }
}

async fn connect_github_account(auth: SmbAuthorization) -> Result<CommandResult> {
    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to link your GitHub account?")
        .interact()
        .unwrap();

    // Link GitHub account if user confirms
    if !confirm {
        let spinner = Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            style("Cancel operation.").green().bold().to_string(),
        );
        return Ok(CommandResult {
            spinner,
            symbol: "✅".to_owned(),
            msg: "Doing nothing.".to_owned(),
        });
    }

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Linking your GitHub account...")
            .green()
            .bold()
            .to_string(),
    );

    let response = Client::new()
        .post(build_smb_connect_github_url())
        .json(&auth)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => Ok(CommandResult {
            spinner,
            symbol: "✅".to_owned(),
            msg: "GitHub account linked!".to_owned(),
        }),
        _ => {
            let error = anyhow!("Failed to link GitHub account.");
            Err(error)
        }
    }
}

async fn login_with_email() -> Result<CommandResult> {
    println!("Provide your login credentials.");
    let username = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Email")
        .validate_with(|email: &String| email_validation(email))
        .interact()
        .unwrap();
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Password")
        .interact()
        .unwrap();
    do_process_login(LoginArgs { username, password }).await
}

async fn do_process_login(args: LoginArgs) -> Result<CommandResult> {
    let login_params = LoginParams {
        user: UserParam {
            email: args.username,
            password: args.password,
        },
    };

    let response = Client::new()
        .post(build_smb_login_url())
        .json(&login_params)
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => {
            // Login successful
            save_token(&response).await?;
            Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    style("Loading...").green().bold().to_string(),
                ),
                symbol: "✅".to_owned(),
                msg: "You are now logged in!".to_owned(),
            })
        }
        StatusCode::NOT_FOUND => {
            // Account not found and we show signup option
            Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    style("Account not found.").green().bold().to_string(),
                ),
                symbol: "✅".to_owned(),
                msg: "Please signup!".to_owned(),
            })
        }
        StatusCode::UNPROCESSABLE_ENTITY => {
            // Account found but email not verified / password not set
            let result: SmbAuthorization = response.json().await?;
            // println!("Result: {:#?}", &result);
            verify_or_set_password(result).await
        }
        _ => Err(anyhow!("Login failed. Check your username and password.")),
    }
}

async fn verify_or_set_password(result: SmbAuthorization) -> Result<CommandResult> {
    match result.error_code {
        Some(error_code) => {
            debug!("{}", error_code);
            match error_code {
                ErrorCode::EmailUnverified => send_email_verification(result.user).await,
                ErrorCode::PasswordNotSet => send_reset_password(result.user).await,
                _ => Err(anyhow!("Shouldn't be here.")),
            }
        }
        None => Err(anyhow!("Shouldn't be here.")),
    }
}

async fn send_reset_password(user: Option<User>) -> Result<CommandResult> {
    // Return early if user is null
    if let Some(user) = user {
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to reset your password?")
            .interact()
            .unwrap();

        // Send verification email if user confirms
        if !confirm {
            let spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Cancel operation.").green().bold().to_string(),
            );
            return Ok(CommandResult {
                spinner,
                symbol: "✅".to_owned(),
                msg: "Doing nothing.".to_owned(),
            });
        }
        resend_reset_password_instruction(user).await
    } else {
        let error = anyhow!("Failed to get user.");
        Err(error)
    }
}

async fn resend_reset_password_instruction(user: User) -> Result<CommandResult> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Sending reset password instruction...")
            .green()
            .bold()
            .to_string(),
    );
    let response = Client::new()
        .post(build_smb_resend_reset_password_instructions_url())
        .body(format!("id={}", user.id))
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => {
            spinner.stop_and_persist(
                "✅",
                "Reset password instruction sent! Please check your email.".to_owned(),
            );
            input_reset_password_token().await
        }
        _ => {
            let error = anyhow!("Failed to send reset password instruction.");
            Err(error)
        }
    }
}

async fn input_reset_password_token() -> Result<CommandResult> {
    let token = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Input reset password token")
        .interact()
        .unwrap();
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("New password.")
        .with_confirmation("Repeat password.", "Error: the passwords don't match.")
        .interact()
        .unwrap();

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Resetting password...").green().bold().to_string(),
    );

    let password_confirmation = password.clone();

    let params = Param {
        user: UserUpdatePassword {
            reset_password_token: token,
            password,
            password_confirmation,
        },
    };

    let response = Client::new()
        .put(build_smb_reset_password_url())
        .json(&params)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => Ok(CommandResult {
            spinner,
            symbol: "✅".to_owned(),
            msg: "Password reset!".to_owned(),
        }),
        _ => {
            let error = anyhow!("Failed to reset password.");
            Err(error)
        }
    }
}

async fn do_process_logout() -> Result<()> {
    let token = get_smb_token().await?;

    let response = Client::new()
        .delete(build_smb_logout_url())
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

fn build_smb_login_url() -> String {
    let mut url_builder = smb_base_url_builder();
    url_builder.add_route("v1/users/sign_in");
    url_builder.build()
}

fn build_smb_logout_url() -> String {
    let mut url_builder = smb_base_url_builder();
    url_builder.add_route("v1/users/sign_out");
    url_builder.build()
}

fn build_smb_resend_email_verification_url() -> String {
    let mut url_builder = smb_base_url_builder();
    url_builder.add_route("v1/resend_confirmation");
    url_builder.build()
}

fn build_smb_resend_reset_password_instructions_url() -> String {
    let mut url_builder = smb_base_url_builder();
    url_builder.add_route("v1/resend_reset_password_instructions");
    url_builder.build()
}

fn build_smb_reset_password_url() -> String {
    let mut url_builder = smb_base_url_builder();
    url_builder.add_route("v1/users/password");
    url_builder.build()
}

fn build_smb_connect_github_url() -> String {
    let mut url_builder = smb_base_url_builder();
    url_builder.add_route("v1/link_github_account");
    url_builder.build()
}
