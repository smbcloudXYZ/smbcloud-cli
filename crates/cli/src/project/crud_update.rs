use crate::token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in};
use anyhow::{anyhow, Result};
use dialoguer::{console::Term, theme::ColorfulTheme, Input, Select};
use smbcloud_model::runner::Runner;
use smbcloud_network::environment::Environment;
use smbcloud_networking::smb_client::SmbClient;
use smbcloud_networking_project::{
    crud_project_read::get_project, crud_project_update::update_project,
};
use spinners::Spinner;

use crate::{
    account::login::process_login,
    cli::CommandResult,
    ui::{description, succeed_message, succeed_symbol},
};

pub async fn process_project_update(env: Environment, project_id: String) -> Result<CommandResult> {
    // Check credentials.
    let is_logged_in = is_logged_in(env).await?;
    if !is_logged_in {
        let _ = process_login(env, Some(is_logged_in)).await;
    }

    let access_token = get_smb_token(env)?;
    let project = get_project(
        env,
        SmbClient::Cli,
        access_token.clone(),
        project_id.clone(),
    )
    .await?;

    if let Some(project_description) = project.description {
        println!("Description: {}", description(&project_description));
    }
    println!("Runner: {}", project.runner);

    // Prompt for new description
    let new_description = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("New description")
        .interact()
    {
        Ok(desc) => desc,
        Err(_) => return Err(anyhow!("Invalid description.")),
    };

    let runners = vec![Runner::NodeJs, Runner::Swift, Runner::Ruby];
    let runner = Select::with_theme(&ColorfulTheme::default())
        .items(&runners)
        .default(0)
        .interact_on_opt(&Term::stderr())
        .map(|i| runners[i.unwrap()])
        .unwrap();

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Loading"),
    );
    update_project(
        env,
        SmbClient::Cli,
        access_token,
        project_id,
        &new_description,
        runner,
    )
    .await?;
    Ok(CommandResult {
        spinner,
        symbol: succeed_symbol(),
        msg: succeed_message("Updated."),
    })
}
