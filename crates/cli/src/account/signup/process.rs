use {
    super::SignupMethod,
    crate::{
        account::lib::authorize_github,
        cli::CommandResult,
        token::smb_token_file_path::smb_token_file_path,
        ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    dialoguer::{console::Term, theme::ColorfulTheme, Input, Password, Select},
    log::debug,
    reqwest::{Client, StatusCode},
    serde::Serialize,
    smbcloud_model::signup::SignupResult,
    smbcloud_network::environment::Environment,
    smbcloud_networking::{
        constants::{PATH_USERS, SMB_USER_AGENT},
        smb_base_url_builder,
    },
    smbcloud_networking_account::signup::signup,
    smbcloud_utils::email_validation,
    spinners::Spinner,
};

pub async fn process_signup(env: Environment) -> Result<CommandResult> {
    // Check if token file exists
    if smb_token_file_path(env).is_some() {
        return Ok(CommandResult {
            spinner: Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Loading"),
            ),
            symbol: fail_symbol(),
            msg: fail_message("You are already logged in. Please logout first."),
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
        SignupMethod::Email => signup_with_email(env, None).await,
        SignupMethod::GitHub => signup_with_github(env).await,
    }
}

pub async fn signup_with_email(env: Environment, email: Option<String>) -> Result<CommandResult> {
    let email = if let Some(email) = email {
        email
    } else {
        Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Username")
            .validate_with(|email: &String| email_validation(email))
            .interact()
            .unwrap()
    };
    signup_input_password_step(env, email).await
}

async fn signup_input_password_step(env: Environment, email: String) -> Result<CommandResult> {
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
        succeed_message("Signing up"),
    );

    match do_signup_email(env, email, password).await {
        Ok(_) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("You are signed up! Check your email to confirm your account."),
        }),
        Err(e) => Ok(CommandResult {
            spinner,
            symbol: fail_symbol(),
            msg: fail_message(&format!("{e}")),
        }),
    }
}

async fn signup_with_github(env: Environment) -> Result<CommandResult> {
    match authorize_github(&env).await {
        Ok(code) => {
            debug!("Code: {:#?}", code);
            Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::BouncingBall,
                    succeed_message("Requesting GitHub token"),
                ),
                symbol: succeed_symbol(),
                msg: succeed_message("Finished requesting GitHub token!"),
            })
        }
        Err(e) => {
            let error = anyhow!("Failed to get code from channel: {e}");
            Err(error)
        }
    }
}

pub async fn do_signup<T: Serialize + ?Sized>(env: Environment, args: &T) -> Result<CommandResult> {
    let spinner = Spinner::new(
        spinners::Spinners::BouncingBall,
        succeed_message("Signing you up"),
    );

    let response = Client::new()
        .post(build_smb_signup_url(env))
        .json(&args)
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message(
                "Your account has been created. Check email for verification link.",
            ),
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

fn build_smb_signup_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS);
    url_builder.build()
}

pub async fn do_signup_email(
    env: Environment,
    email: String,
    password: String,
) -> Result<CommandResult> {
    let spinner = Spinner::new(
        spinners::Spinners::BouncingBall,
        succeed_message("Signing you up"),
    );

    match signup(env, SMB_USER_AGENT.to_string(), email, password).await {
        Ok(_) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message(
                "Your account has been created. Check email for verification link.",
            ),
        }),
        Err(e) => {
            let error = anyhow!("Failed to signup: {}", e);
            Err(error)
        }
    }
}
