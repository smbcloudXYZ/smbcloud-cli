use crate::account::lib::is_logged_in;
use crate::client;
use crate::token::get_smb_token::get_smb_token;
use crate::{
    cli::CommandResult,
    ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
};
use anyhow::Result;
use smbcloud_model::account::User;
use smbcloud_network::environment::Environment;
use smbcloud_networking_account::me::me;
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
    if !is_logged_in(env) {
        return Ok(CommandResult {
            spinner: Spinner::new(
                spinners::Spinners::SimpleDotsScrolling,
                succeed_message("Loading"),
            ),
            symbol: fail_symbol(),
            msg: fail_message("You are not logged in. Please login first."),
        });
    }
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );
    let token = get_smb_token(env)?;
    match me(env, client(), &token).await {
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
            println!("Error: {e:#?}");
            Ok(CommandResult {
                spinner,
                symbol: fail_symbol(),
                msg: fail_message("Failed to get all projects."),
            })
        }
    }
}
