use crate::client;
use crate::token::get_smb_token::get_smb_token;
use crate::{
    cli::CommandResult,
    deploy::config::{check_project, get_config},
    ui::{
        deployment_detail_view::show_deployment_detail_tui, deployment_table::show_deployments_tui,
        succeed_message, succeed_symbol,
    },
};
use anyhow::{anyhow, Result};
use smbcloud_network::environment::Environment;
use smbcloud_networking_project::crud_project_deployment_read::{get_deployment, get_deployments};
use spinners::Spinner;

pub(crate) async fn process_deployment(
    env: Environment,
    id: Option<String>,
) -> Result<CommandResult> {
    let mut spinner: Spinner =
        Spinner::new(spinners::Spinners::Hamburger, succeed_message("Loading"));
    // Load project id from .smb/config.toml
    let config = get_config(env, None).await?;

    let access_token = get_smb_token(env)?;

    check_project(env, &access_token, config.project.id).await?;

    if let Some(deployment_id) = id {
        // Show detail for a specific deployment
        let deployment_id: i32 = deployment_id.parse()?;
        let deployment = get_deployment(
            env,
            client(),
            access_token,
            config.project.id,
            deployment_id,
        )
        .await?;
        spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded"));
        show_deployment_detail_tui(&deployment).map_err(|e| anyhow!(e))?;
    } else {
        // List all deployments for the project
        let access_token = get_smb_token(env)?;
        let deployments = get_deployments(env, client(), access_token, config.project.id).await?;
        spinner.stop_and_persist(&succeed_symbol(), succeed_message("Load all deployments"));
        show_deployments_tui(deployments).map_err(|e| anyhow!(e))?;
    };

    Ok(CommandResult {
        spinner: Spinner::new(spinners::Spinners::Hamburger, succeed_message("Loading.")),
        symbol: succeed_symbol(),
        msg: succeed_message("Loaded"),
    })
}
