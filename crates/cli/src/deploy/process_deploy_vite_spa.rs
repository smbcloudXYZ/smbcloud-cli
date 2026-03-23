use {
    crate::{
        cli::CommandResult,
        client,
        deploy::rsync_deploy::rsync_deploy,
        ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    chrono::Utc,
    smbcloud_model::project::{DeploymentPayload, DeploymentStatus},
    smbcloud_network::environment::Environment,
    smbcloud_networking_account::me::me,
    smbcloud_networking_project::{
        crud_project_deployment_create::create_deployment, crud_project_deployment_update::update,
    },
    smbcloud_utils::config::Config,
    spinners::{Spinner, Spinners},
    std::process::Command,
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

    // ── Step 1: build locally ────────────────────────────────────────────────

    let mut build_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        succeed_message(&format!(
            "Building {} with {}…",
            project_path, package_manager
        )),
    );

    // Validate the directory exists before spawning — current_dir silently
    // produces os error 2 at status() time if the path doesn't exist, which
    // gives a confusing "Failed to spawn" message instead of the real cause.
    let project_dir = std::path::Path::new(project_path);
    if !project_dir.exists() {
        build_spinner.stop_and_persist(
            &fail_symbol(),
            fail_message(&format!(
                "Project path '{}' does not exist. Check the 'path' field in .smb/config.toml.",
                project_path
            )),
        );
        return Err(anyhow!(fail_message(&format!(
            "Project path '{}' does not exist.",
            project_path
        ))));
    }

    let build_status = Command::new(package_manager)
        .arg("build")
        .current_dir(project_path)
        .status()
        .map_err(|error| {
            anyhow!(fail_message(&format!(
                "Failed to spawn '{}': {}",
                package_manager, error
            )))
        })?;

    if !build_status.success() {
        build_spinner.stop_and_persist(
            &fail_symbol(),
            fail_message("Build failed. See output above."),
        );
        return Err(anyhow!(fail_message(&format!(
            "'{} build' exited with status {}",
            package_manager, build_status,
        ))));
    }

    build_spinner.stop_and_persist(&succeed_symbol(), succeed_message("Build complete."));

    // ── Step 2: record deployment as Started ─────────────────────────────────
    //
    // A pure rsync SPA deploy has no git commit hash; use a UTC timestamp as a
    // lightweight, non-empty identifier for the API record.

    let deploy_ref = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    let start_payload = DeploymentPayload {
        commit_hash: deploy_ref.clone(),
        status: DeploymentStatus::Started,
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

    let local_dist = format!("{}/{}", project_path, output_dir);
    let runner = config.project.runner;

    match rsync_deploy(&config, &runner, user.id, &local_dist) {
        Ok(_) => {}
        Err(error) => {
            if let Some(ref deployment) = created_deployment {
                let failed_payload = DeploymentPayload {
                    commit_hash: deploy_ref.clone(),
                    status: DeploymentStatus::Failed,
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
            return Err(error);
        }
    }

    // ── Step 4: mark deployment as Done ──────────────────────────────────────

    if let Some(ref deployment) = created_deployment {
        let done_payload = DeploymentPayload {
            commit_hash: deploy_ref,
            status: DeploymentStatus::Done,
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
