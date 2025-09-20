use crate::token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in};
use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Input};
use smbcloud_network::environment::Environment;
use smbcloud_networking_project::{
    crud_project_read::get_project, crud_project_update::update_project_description,
};
use spinners::Spinner;

use crate::{
    account::login::process_login,
    cli::CommandResult,
    ui::{description, succeed_message, succeed_symbol},
};

pub async fn process_project_update_description(
    env: Environment,
    project_id: String,
) -> Result<CommandResult> {
    // Check credentials.
    let is_logged_in = is_logged_in(env).await?;
    if !is_logged_in {
        let _ = process_login(env, Some(is_logged_in)).await;
    }

    let access_token = get_smb_token(env)?;
    let project = get_project(env, access_token.clone(), project_id.clone()).await?;

    if let Some(project_description) = project.description {
        println!("Description: {}", description(&project_description));
    }

    // Prompt for new description
    let new_description = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("New description")
        .interact()
    {
        Ok(desc) => desc,
        Err(_) => return Err(anyhow!("Invalid description.")),
    };
    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );
    update_project_description(env, access_token, project_id, &new_description).await?;
    Ok(CommandResult {
        spinner,
        symbol: succeed_symbol(),
        msg: succeed_message("Updated."),
    })
}
