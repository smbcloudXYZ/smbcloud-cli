use {
    crate::{
        account::{
            lib::{authorize_github, store_token},
            signup::{do_signup, signup_with_email, SignupMethod},
        },
        cli::CommandResult,
        token::is_logged_in::is_logged_in as is_logged_in_async,
        ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    console::style,
    dialoguer::{console::Term, theme::ColorfulTheme, Confirm, Input, Password, Select},
    log::debug,
    reqwest::{Client, StatusCode},
    smbcloud_model::{
        account::{
            ErrorCode::{
                self as AccountErrorCode, EmailNotFound, EmailUnverified, GithubNotLinked,
                PasswordNotSet,
            },
            GithubInfo, SmbAuthorization, User,
        },
        forgot::{Param, UserUpdatePassword},
        login::{AccountStatus, LoginArgs},
        signup::{GithubEmail, Provider, SignupGithubParams, SignupUserGithub},
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking::{
        constants::{PATH_LINK_GITHUB_ACCOUNT, PATH_USERS_PASSWORD},
        smb_base_url_builder,
        smb_client::SmbClient,
    },
    smbcloud_networking_account::{
        check_email::check_email, login::login,
        resend_email_verification::resend_email_verification as account_resend_email_verification,
        resend_reset_password_instruction::resend_reset_password_instruction as account_resend_reset_password_instruction,
    },
    smbcloud_utils::email_validation,
    spinners::Spinner,
};

pub async fn process_login(env: Environment, is_logged_in: Option<bool>) -> Result<CommandResult> {
    let should_continue = match is_logged_in {
        Some(is_logged_id) => !is_logged_id,
        None => {
            // Check if logged in
            let logged_in = is_logged_in_async(env).await?;
            !logged_in
        }
    };

    if !should_continue {
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
            EmailNotFound => return create_new_account(env, auth.user_email, auth.user_info).await,
            EmailUnverified => return send_email_verification(env, auth.user).await,
            PasswordNotSet => {
                // Only for email and password login
                let error = anyhow!("Password not set.");
                return Err(error);
            }
            GithubNotLinked => return connect_github_account(env, auth).await,
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
                symbol: succeed_symbol(),
                msg: succeed_message("Doing nothing."),
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
    match account_resend_email_verification(env, SmbClient::Cli, user.email).await {
        Ok(_) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Verification email sent!"),
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
            succeed_message("Cancel operation."),
        );
        return Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Doing nothing."),
        });
    }

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Linking your GitHub account..."),
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
            symbol: succeed_symbol(),
            msg: succeed_message("GitHub account linked!"),
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

    match check_email(env, SmbClient::Cli, &username).await {
        Ok(auth) => {
            // Only continue with password input if email is found and confirmed.
            if auth.error_code.is_some() {
                // Check if email is in the database, unconfirmed. Only presents password input if email is found and confirmed.
                let spinner = Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    succeed_message("Checking email"),
                );
                after_checking_email_step(&env, spinner, auth, Some(username)).await
            } else {
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
        }
        Err(_) => Err(anyhow!(fail_message(
            "Server error. Please try again later."
        ))),
    }
}

async fn do_process_login(env: Environment, args: LoginArgs) -> Result<CommandResult> {
    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );
    let account_status = match login(env, SmbClient::Cli, args.username, args.password).await {
        Ok(response) => response,
        Err(_) => return Err(anyhow!(fail_message("Check your internet connection."))),
    };

    match account_status {
        AccountStatus::Ready { access_token } => {
            store_token(env, access_token).await?;
            Ok(CommandResult {
                spinner,
                symbol: succeed_symbol(),
                msg: succeed_message("You are logged in!"),
            })
        }
        AccountStatus::NotFound => Err(anyhow!(fail_message("Check your internet connection."))),
        AccountStatus::Incomplete { status } => {
            action_on_account_status(&env, spinner, status, None, None).await
        }
    }
}

async fn after_checking_email_step(
    env: &Environment,
    mut spinner: Spinner,
    result: SmbAuthorization,
    username: Option<String>,
) -> Result<CommandResult> {
    match result.error_code {
        Some(error_code) => {
            debug!("{}", error_code);
            match error_code {
                EmailNotFound => {
                    spinner.stop_and_persist(
                        &succeed_symbol(),
                        succeed_message(
                            "Account not found. Please continue with setting up your account.",
                        ),
                    );
                    signup_with_email(env.to_owned(), username).await
                }
                EmailUnverified => {
                    spinner.stop_and_persist(
                        &succeed_symbol(),
                        succeed_message("Email not verified. Please verify your email."),
                    );
                    send_email_verification(*env, result.user).await
                }
                PasswordNotSet => {
                    spinner.stop_and_persist(
                        &succeed_symbol(),
                        succeed_message("Password not set. Please reset your password."),
                    );
                    send_reset_password(*env, result.user).await
                }
                _ => {
                    spinner.stop_and_persist(&fail_symbol(), fail_message("An error occurred."));
                    Err(anyhow!("Idk what happened."))
                }
            }
        }
        None => Err(anyhow!("Shouldn't be here.")),
    }
}

async fn action_on_account_status(
    env: &Environment,
    mut spinner: Spinner,
    error_code: AccountErrorCode,
    username: Option<String>,
    user: Option<User>,
) -> Result<CommandResult> {
    match error_code {
        EmailNotFound => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("Account not found. Please continue with setting up your account."),
            );
            signup_with_email(env.to_owned(), username).await
        }
        EmailUnverified => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("Email not verified. Please verify your email."),
            );
            send_email_verification(*env, user).await
        }
        PasswordNotSet => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("Password not set. Please reset your password."),
            );
            send_reset_password(*env, user).await
        }
        _ => {
            spinner.stop_and_persist(&fail_symbol(), fail_message("An error occurred."));
            Err(anyhow!("Idk what happened."))
        }
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
                symbol: succeed_symbol(),
                msg: succeed_message("Doing nothing."),
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
        succeed_message("Sending reset password instruction..."),
    );

    match account_resend_reset_password_instruction(env, SmbClient::Cli, user.email).await {
        Ok(_) => {
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

fn build_smb_reset_password_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env, &SmbClient::Cli);
    url_builder.add_route(PATH_USERS_PASSWORD);
    url_builder.build()
}

fn build_smb_connect_github_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env, &SmbClient::Cli);
    url_builder.add_route(PATH_LINK_GITHUB_ACCOUNT);
    url_builder.build()
}
