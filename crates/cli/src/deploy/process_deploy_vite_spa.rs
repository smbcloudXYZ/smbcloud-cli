use {
    crate::{
        cli::CommandResult,
        client,
        ui::{succeed_message, succeed_symbol},
    },
    anyhow::Result,
    chrono::Utc,
    smbcloud_auth::me::me,
    smbcloud_deploy::{BuildStrategy, Transport},
    smbcloud_model::project::{DeploymentPayload, DeploymentStatus},
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::{
        crud_project_deployment_create::create_deployment, crud_project_deployment_update::update,
    },
    smbcloud_utils::config::Config,
    spinners::{Spinner, Spinners},
};

pub async fn process_deploy_vite_spa(env: Environment, config: Config) -> Result<CommandResult> {
    // Resolve required SPA fields from the project config.
    // `source` is the local directory containing the vite project (e.g. "frontend/connected-devices/").
    // `path` is the remote destination on the server, consumed by rsync_deploy.
    let project_path = config.project.source.as_deref().unwrap_or(".");
    let output_dir = config.project.output.as_deref().unwrap_or("dist");
    let package_manager = config.project.package_manager.as_deref().unwrap_or("pnpm");

    // Fetch user for SSH key and deployment record.
    let access_token = crate::token::get_smb_token::get_smb_token(env)?;
    let user = me(env, client(), &access_token).await?;

    // ── Step 1: build locally (engine BuildStrategy) ─────────────────────────

    let reporter = crate::ui::reporter::SpinnerReporter::new();
    let artifact = smbcloud_deploy::ViteSpaBuild {
        project_path: project_path.to_string(),
        output_dir: output_dir.to_string(),
        package_manager: package_manager.to_string(),
    }
    .build(&reporter)?;

    // ── Step 2: record deployment as Started ─────────────────────────────────
    //
    // A pure rsync SPA deploy has no git commit hash; use a UTC timestamp as a
    // lightweight, non-empty identifier for the API record.

    let deploy_ref = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    let start_payload = DeploymentPayload {
        commit_hash: deploy_ref.clone(),
        status: DeploymentStatus::Started,
        frontend_app_id: config.project.frontend_app_id.clone(),
    };

    let created_deployment = create_deployment(
        env,
        client(),
        &access_token,
        config.project.id,
        start_payload,
    )
    .await
    .ok();

    // ── Step 3: rsync <project_path>/<output_dir>/ to api.smbcloud.xyz ───────
    //
    // config.project.path holds the remote destination on the server
    // (e.g. "apps/web/myapp"). rsync_deploy appends a
    // trailing slash and targets git@api.smbcloud.xyz:<path>/ using the
    // pinned known-hosts and the user's smbCloud SSH key — exactly the same
    // transport used for static site deployments.

    let runner = config.project.runner;
    let transport = crate::deploy::rsync_transport(&config, &runner, user.id)?;
    match transport.ship(&artifact.source_dir, &reporter) {
        Ok(()) => {}
        Err(error) => {
            if let Some(ref deployment) = created_deployment {
                let failed_payload = DeploymentPayload {
                    commit_hash: deploy_ref.clone(),
                    status: DeploymentStatus::Failed,
                    frontend_app_id: config.project.frontend_app_id.clone(),
                };
                let _ = update(
                    env,
                    client(),
                    access_token.clone(),
                    config.project.id,
                    deployment.id,
                    failed_payload,
                )
                .await;
            }
            return Err(error.into());
        }
    }

    // ── Step 4: mark deployment as Done ──────────────────────────────────────

    if let Some(ref deployment) = created_deployment {
        let done_payload = DeploymentPayload {
            commit_hash: deploy_ref,
            status: DeploymentStatus::Done,
            frontend_app_id: config.project.frontend_app_id.clone(),
        };
        match update(
            env,
            client(),
            access_token,
            config.project.id,
            deployment.id,
            done_payload,
        )
        .await
        {
            Ok(_) => println!("App is running {}", succeed_symbol()),
            Err(update_err) => {
                eprintln!("Error updating deployment status to Done: {}", update_err)
            }
        }
    }

    let spinner = Spinner::new(Spinners::Hamburger, succeed_message("Deployment complete."));
    Ok(CommandResult {
        spinner,
        symbol: succeed_symbol(),
        msg: succeed_message("Deployment complete."),
    })
}
