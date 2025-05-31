use crate::{
    cli::CommandResult,
    deploy::config::{check_config, check_project},
    ui::{succeed_message, succeed_symbol},
};
use anyhow::Result;
use smbcloud_model::project::Deployment;
use smbcloud_networking::{environment::Environment, get_smb_token};
use smbcloud_networking_project::crud_project_deployment_read::{
    get_deployment_detail, list_deployments,
};
use spinners::Spinner;
use tabled::{Table, Tabled};

pub(crate) async fn process_deployment(
    env: Environment,
    id: Option<String>,
) -> Result<CommandResult> {
    let mut spinner: Spinner =
        Spinner::new(spinners::Spinners::Hamburger, succeed_message("Loading"));
    // Load project id from .smb/config.toml
    let config = check_config(env).await?;

    let access_token = get_smb_token(env).await?;

    check_project(env,access_token, config.project.id).await?;

    if let Some(deployment_id) = id {
        // Show detail for a specific deployment
        let deployment_id: i32 = deployment_id.parse()?;
        let deployment = get_deployment_detail(env, config.project.id, deployment_id).await?;
        spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded"));
        show_deployment_detail(&deployment);
    } else {
        // List all deployments for the project
        let access_token = get_smb_token(env).await?;
        let deployments = list_deployments(env, access_token, config.project.id).await?;
        spinner.stop_and_persist(&succeed_symbol(), succeed_message("Load all deployments"));
        show_project_deployments(&deployments);
    };

    Ok(CommandResult {
        spinner: Spinner::new(spinners::Spinners::Hamburger, succeed_message("Loading.")),
        symbol: succeed_symbol(),
        msg: succeed_message("Loaded"),
    })
}

// Helper struct for table display
#[derive(Tabled)]
struct DeploymentRow {
    id: i32,
    commit_hash: String,
    status: String,
    created_at: String,
    updated_at: String,
}

pub fn show_project_deployments(deployments: &[Deployment]) {
    let rows: Vec<DeploymentRow> = deployments
        .iter()
        .map(|d| DeploymentRow {
            id: d.id,
            commit_hash: d.commit_hash.clone(),
            status: format!("{:?}", d.status),
            created_at: d.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: d.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect();

    let table = Table::new(rows);
    println!("{table}");
}

pub fn show_deployment_detail(deployment: &Deployment) {
    #[derive(Tabled)]
    struct Detail {
        #[tabled(rename = "ID")]
        id: i32,
        #[tabled(rename = "Project ID")]
        project_id: i32,
        #[tabled(rename = "Commit Hash")]
        commit_hash: String,
        #[tabled(rename = "Status")]
        status: String,
        #[tabled(rename = "Created At")]
        created_at: String,
        #[tabled(rename = "Updated At")]
        updated_at: String,
    }

    let row = Detail {
        id: deployment.id,
        project_id: deployment.project_id,
        commit_hash: deployment.commit_hash.clone(),
        status: format!("{:?}", deployment.status),
        created_at: deployment
            .created_at
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        updated_at: deployment
            .updated_at
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    };

    let table = Table::new(vec![row]);
    println!("{table}");
}
