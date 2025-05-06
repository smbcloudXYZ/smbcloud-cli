use crate::{
    account::{
        lib::{authorize_github, save_token},
        signup::{do_signup, SignupMethod},
    },
    cli::CommandResult,
    ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use log::debug;
use reqwest::{Client, StatusCode};
use smbcloud_model::{
    account::{ErrorCode, GithubInfo, SmbAuthorization, User},
    forgot::{Param, UserUpdatePassword},
    login::{LoginArgs, LoginParams, UserParam},
    signup::{GithubEmail, Provider, SignupGithubParams, SignupUserGithub},
};
use smbcloud_networking::{
    constants::{
        PATH_LINK_GITHUB_ACCOUNT, PATH_RESEND_CONFIRMATION, PATH_RESET_PASSWORD_INSTRUCTIONS,
        PATH_USERS_PASSWORD, PATH_USERS_SIGN_IN,
    },
    environment::Environment,
    smb_base_url_builder, smb_token_file_path,
};
use smbcloud_utils::email_validation;
use spinners::Spinner;

pub async fn process_login(env: Environment) -> Result<CommandResult> {
    // Check if token file exists
    if smb_token_file_path(env).is_some() {
        return Ok(CommandResult {
            spinner: Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Loading...").green().bold().to_string(),
            ),
            symbol: fail_symbol(),
            msg: fail_message("You are already logged in. Please logout first."),
        });
    }

    let signup_methods = vec![SignupMethod::Email, SignupMethod::GitHub];
    let selection = match Select::with_theme(&ColorfulTheme::default())
        .items(&signup_methods)
        .default(0)
        .interact_on_opt(&Term::stderr())
        .map(|i| signup_methods[i.unwrap()])
    {
        Ok(method) => method,
        Err(_) => {
            let error = anyhow!("No selection made.");
            return Err(error);
        }
    };

    match selection {
        SignupMethod::Email => login_with_email(env).await,
        SignupMethod::GitHub => login_with_github(env).await,
    }
}

// Private functions

async fn login_with_github(env: Environment) -> Result<CommandResult> {
    match authorize_github(&env).await {
        Ok(result) => process_authorization(env, result).await,
        Err(err) => {
            let error = anyhow!("Failed to authorize your GitHub account. {}", err);
            Err(error)
        }
    }
}

async fn process_authorization(env: Environment, auth: SmbAuthorization) -> Result<CommandResult> {
    // What to do if not logged in with GitHub?
    // Check error_code first
    if let Some(error_code) = auth.error_code {
        debug!("{}", error_code);
        match error_code {
            ErrorCode::EmailNotFound => {
                return create_new_account(env, auth.user_email, auth.user_info).await
            }
            ErrorCode::EmailUnverified => return send_email_verification(env, auth.user).await,
            ErrorCode::PasswordNotSet => {
                // Only for email and password login
                let error = anyhow!("Password not set.");
                return Err(error);
            }
            ErrorCode::GithubNotLinked => return connect_github_account(env, auth).await,
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
    env: Environment,
    user_email: Option<GithubEmail>,
    user_info: Option<GithubInfo>,
) -> Result<CommandResult> {
    let confirm = match Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to create a new account?")
        .interact()
    {
        Ok(confirm) => confirm,
        Err(_) => {
            let error = anyhow!("Invalid input.");
            return Err(error);
        }
    };

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

        return do_signup(env, &params).await;
    }

    Err(anyhow!("Shouldn't be here."))
}

async fn send_email_verification(env: Environment, user: Option<User>) -> Result<CommandResult> {
    // Return early if user is null
    if let Some(user) = user {
        let confirm = match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to send a new verification email?")
            .interact()
        {
            Ok(confirm) => confirm,
            Err(_) => {
                let error = anyhow!("Invalid input.");
                return Err(error);
            }
        };

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
        resend_email_verification(env, user).await
    } else {
        let error = anyhow!("Failed to get user.");
        Err(error)
    }
}

async fn resend_email_verification(env: Environment, user: User) -> Result<CommandResult> {
    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Sending verification email...")
            .green()
            .bold()
            .to_string(),
    );

    let response = Client::new()
        .post(build_smb_resend_email_verification_url(env))
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

async fn connect_github_account(env: Environment, auth: SmbAuthorization) -> Result<CommandResult> {
    let confirm = match Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to link your GitHub account?")
        .interact()
    {
        Ok(confirm) => confirm,
        Err(_) => {
            let error = anyhow!("Invalid input.");
            return Err(error);
        }
    };

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
        .post(build_smb_connect_github_url(env))
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

async fn login_with_email(env: Environment) -> Result<CommandResult> {
    println!("Provide your login credentials.");
    let username = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Email")
        .validate_with(|email: &String| email_validation(email))
        .interact()
    {
        Ok(email) => email,
        Err(_) => {
            let error = anyhow!("Invalid email.");
            return Err(error);
        }
    };
    let password = match Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Password")
        .interact()
    {
        Ok(password) => password,
        Err(_) => {
            let error = anyhow!("Invalid password.");
            return Err(error);
        }
    };
    do_process_login(env, LoginArgs { username, password }).await
}

async fn do_process_login(env: Environment, args: LoginArgs) -> Result<CommandResult> {
    let login_params = LoginParams {
        user: UserParam {
            email: args.username,
            password: args.password,
        },
    };

    let response = match Client::new()
        .post(build_smb_login_url(env))
        .json(&login_params)
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return Err(anyhow!(fail_message("Check your internet connection."))),
    };

    match response.status() {
        StatusCode::OK => {
            // Login successful
            save_token(env, &response).await?;
            Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    style("Loading...").green().bold().to_string(),
                ),
                symbol: succeed_symbol(),
                msg: succeed_message("You are logged in!"),
            })
        }
        StatusCode::NOT_FOUND => {
            // Account not found and we show signup option
            Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    style("Account not found.").green().bold().to_string(),
                ),
                symbol: fail_symbol(),
                msg: fail_message("Please signup!"),
            })
        }
        StatusCode::UNPROCESSABLE_ENTITY => {
            // Account found but email not verified / password not set
            let result: SmbAuthorization = response.json().await?;
            // println!("Result: {:#?}", &result);
            verify_or_set_password(&env, result).await
        }
        _ => Err(anyhow!(fail_message(
            "Login failed. Check your username and password."
        ))),
    }
}

async fn verify_or_set_password(
    env: &Environment,
    result: SmbAuthorization,
) -> Result<CommandResult> {
    match result.error_code {
        Some(error_code) => {
            debug!("{}", error_code);
            match error_code {
                ErrorCode::EmailUnverified => send_email_verification(*env, result.user).await,
                ErrorCode::PasswordNotSet => send_reset_password(*env, result.user).await,
                _ => Err(anyhow!("Shouldn't be here.")),
            }
        }
        None => Err(anyhow!("Shouldn't be here.")),
    }
}

async fn send_reset_password(env: Environment, user: Option<User>) -> Result<CommandResult> {
    // Return early if user is null
    if let Some(user) = user {
        let confirm = match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to reset your password?")
            .interact()
        {
            Ok(confirm) => confirm,
            Err(_) => {
                let error = anyhow!("Invalid input.");
                return Err(error);
            }
        };

        // Send verification email if user confirms
        if !confirm {
            let spinner = Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                style("Cancel operation.").green().bold().to_string(),
            );
            return Ok(CommandResult {
                spinner,
                symbol: style("✔").green().to_string(),
                msg: "Doing nothing.".to_owned(),
            });
        }
        resend_reset_password_instruction(env, user).await
    } else {
        let error = anyhow!("Failed to get user.");
        Err(error)
    }
}

async fn resend_reset_password_instruction(env: Environment, user: User) -> Result<CommandResult> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Sending reset password instruction...")
            .green()
            .bold()
            .to_string(),
    );
    let response = Client::new()
        .post(build_smb_resend_reset_password_instructions_url(env))
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
            input_reset_password_token(env).await
        }
        _ => {
            let error = anyhow!("Failed to send reset password instruction.");
            Err(error)
        }
    }
}

async fn input_reset_password_token(env: Environment) -> Result<CommandResult> {
    let token = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Input reset password token")
        .interact()
    {
        Ok(token) => token,
        Err(_) => {
            let error = anyhow!("Invalid token.");
            return Err(error);
        }
    };
    let password = match Password::with_theme(&ColorfulTheme::default())
        .with_prompt("New password.")
        .with_confirmation("Repeat password.", "Error: the passwords don't match.")
        .interact()
    {
        Ok(password) => password,
        Err(_) => {
            let error = anyhow!("Invalid password.");
            return Err(error);
        }
    };

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
        .put(build_smb_reset_password_url(env))
        .json(&params)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Password reset!"),
        }),
        _ => Err(anyhow!(fail_message("Failed to reset password."))),
    }
}

fn build_smb_login_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_SIGN_IN);
    url_builder.build()
}

fn build_smb_resend_email_verification_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_RESEND_CONFIRMATION);
    url_builder.build()
}

fn build_smb_resend_reset_password_instructions_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_RESET_PASSWORD_INSTRUCTIONS);
    url_builder.build()
}

fn build_smb_reset_password_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_PASSWORD);
    url_builder.build()
}

fn build_smb_connect_github_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_LINK_GITHUB_ACCOUNT);
    url_builder.build()
}
