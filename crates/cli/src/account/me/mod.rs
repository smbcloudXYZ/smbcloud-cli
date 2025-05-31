use crate::{
    cli::CommandResult,
    ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use smbcloud_model::account::User;
use smbcloud_networking::{
    constants::PATH_USERS_ME, environment::Environment, get_smb_token, smb_base_url_builder,
};
use spinners::Spinner;
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct UserRow {
    #[tabled(rename = "ID")]
    id: i32,
    #[tabled(rename = "Email")]
    email: String,
    #[tabled(rename = "Created At")]
    created_at: String,
    #[tabled(rename = "Updated At")]
    updated_at: String,
}

fn show_user(user: &User) {
    let row = UserRow {
        id: user.id,
        email: user.email.clone(),
        created_at: user.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        updated_at: user.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    let table = Table::new(vec![row]);
    println!("{table}");
}

pub async fn process_me(env: Environment) -> Result<CommandResult> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );
    match me(env).await {
        Ok(user) => {
            spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
            show_user(&user);
            Ok(CommandResult {
                spinner: Spinner::new(
                    spinners::Spinners::SimpleDotsScrolling,
                    succeed_message("Loading"),
                ),
                symbol: succeed_symbol(),
                msg: succeed_message("User info loaded."),
            })
        }
        Err(e) => {
            spinner.stop_and_persist(
                &fail_symbol(),
                fail_message("Error while requesting your information."),
            );
            Err(e)
        }
    }
}

pub async fn me(env: Environment) -> Result<User> {
    let token = get_smb_token(env).await?;

    let response = Client::new()
        .get(build_smb_info_url(env))
        .header("Authorization", token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => {
            let user: User = response.json().await?;
            Ok(user)
        }
        StatusCode::UNAUTHORIZED => Err(anyhow!("Invalid token. Please logout first.")),
        _ => Err(anyhow!("Error while requesting your information.")),
    }
}

fn build_smb_info_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_ME);
    url_builder.build()
}
