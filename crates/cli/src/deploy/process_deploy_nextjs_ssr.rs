use {
    crate::{
        cli::CommandResult,
        client,
        deploy::known_hosts,
        ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    chrono::Utc,
    smbcloud_auth::me::me,
    smbcloud_model::project::{DeploymentPayload, DeploymentStatus},
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::{
        crud_project_deployment_create::create_deployment, crud_project_deployment_update::update,
    },
    smbcloud_utils::config::Config,
    spinners::{Spinner, Spinners},
    std::{
        io::Write,
        process::{Command, Stdio},
    },
    tempfile::NamedTempFile,
};

/// Deploys a Next.js SSR app using standalone output mode.
///
/// Requires `output: 'standalone'` in next.config.js. This produces a
/// self-contained `.next/standalone/` directory that includes only the
/// production Node.js dependencies needed to run the server — no
/// `node_modules` transfer required.
///
/// Steps:
///   1. `pnpm install --ignore-scripts`
///   2. `pnpm build`
///   3. POST deployment record as Started
///   4. rsync .next/standalone/  → server:path/          (server + bundled deps)
///   5. rsync .next/static/      → server:path/.next/static/  (static chunks)
///   6. rsync public/            → server:path/public/   (public assets)
///   7. SSH: pm2 restart/start `node server.js`
///   8. PATCH deployment record as Done
pub async fn process_deploy_nextjs_ssr(env: Environment, config: Config) -> Result<CommandResult> {
    let source = config.project.source.as_deref().unwrap_or(".");
    let package_manager = config.project.package_manager.as_deref().unwrap_or("pnpm");

    let remote_path = config.project.path.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "path not set in .smb/config.toml (e.g. path = \"apps/web/myapp\")"
        ))
    })?;

    let pm2_app = config.project.pm2_app.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "pm2_app not set in .smb/config.toml (e.g. pm2_app = \"my-app\")"
        ))
    })?;

    let access_token = crate::token::get_smb_token::get_smb_token(env)?;
    let user = me(env, client(), &access_token).await?;

    // ── Step 1: pnpm install --ignore-scripts ────────────────────────────────

    let source_dir = std::path::Path::new(source);
    if !source_dir.exists() {
        return Err(anyhow!(fail_message(&format!(
            "Source path '{}' does not exist. Check the 'source' field in .smb/config.toml.",
            source
        ))));
    }

    let mut install_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        succeed_message(&format!("Installing dependencies in {}…", source)),
    );

    // Capture stdout/stderr so pnpm's output does not interleave with the
    // spinner animation. On failure the captured output is printed for the user.
    let install_output = Command::new(package_manager)
        .args(["install", "--ignore-scripts"])
        .current_dir(source)
        .output()
        .map_err(|e| {
            anyhow!(fail_message(&format!(
                "Failed to spawn '{} install': {}",
                package_manager, e
            )))
        })?;

    if !install_output.status.success() {
        install_spinner.stop_and_persist(&fail_symbol(), fail_message("Install failed."));
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        let details = if !stderr.trim().is_empty() {
            stderr
        } else {
            stdout
        };
        if !details.trim().is_empty() {
            eprintln!("{}", details.trim());
        }
        return Err(anyhow!(fail_message(&format!(
            "'{} install --ignore-scripts' exited with status {}",
            package_manager, install_output.status
        ))));
    }

    install_spinner.stop_and_persist(
        &succeed_symbol(),
        succeed_message("Dependencies installed."),
    );

    // ── Step 2: pnpm build ───────────────────────────────────────────────────
    // No spinner — Next.js writes its own rich progress output to the terminal.
    // Blank lines before and after keep it visually separated from our log lines.

    println!();

    let build_status = Command::new(package_manager)
        .arg("build")
        .current_dir(source)
        .status()
        .map_err(|e| {
            anyhow!(fail_message(&format!(
                "Failed to spawn '{} build': {}",
                package_manager, e
            )))
        })?;

    println!();

    if !build_status.success() {
        return Err(anyhow!(fail_message(&format!(
            "'{} build' exited with status {}",
            package_manager, build_status
        ))));
    }

    println!(
        "{} {}",
        succeed_symbol(),
        succeed_message("Build complete.")
    );

    // ── Step 3: verify standalone output exists ──────────────────────────────
    //
    // If output: 'standalone' is missing from next.config.js the build
    // succeeds but .next/standalone/ is never created.

    let standalone_dir = format!("{}/.next/standalone", source);
    if !std::path::Path::new(&standalone_dir).exists() {
        return Err(anyhow!(fail_message(
            ".next/standalone not found. Add `output: 'standalone'` to next.config.js and rebuild."
        )));
    }

    // ── Step 4: record deployment as Started ─────────────────────────────────

    let deploy_ref = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    let created_deployment = create_deployment(
        env,
        client(),
        &access_token,
        config.project.id,
        DeploymentPayload {
            commit_hash: deploy_ref.clone(),
            status: DeploymentStatus::Started,
        },
    )
    .await
    .ok();

    // ── Steps 5–7: rsync three items to the server ───────────────────────────
    //
    // Standalone mode produces everything needed to run the server:
    //
    //   .next/standalone/  — server.js + bundled production deps (no node_modules needed)
    //   .next/static/      — client-side chunks (must be copied into standalone manually)
    //   public/            — static assets served directly by Next.js
    //
    // The static and public dirs must sit inside the standalone tree so
    // `node server.js` can find them at runtime.

    let runner = config.project.runner;
    let rsync_host = runner.rsync_host();

    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let identity_file = home.join(".ssh").join(format!("id_{}@smbcloud", user.id));
    let identity_file_str = identity_file.to_string_lossy().into_owned();

    // Write pinned known_hosts once for all rsync calls.
    let mut known_hosts_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
        .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    let ssh_command = format!(
        "ssh -i {identity} \
         -o StrictHostKeyChecking=yes \
         -o UserKnownHostsFile={known_hosts} \
         -o IdentitiesOnly=yes \
         -o PasswordAuthentication=no \
         -o BatchMode=yes",
        identity = identity_file_str,
        known_hosts = known_hosts_file.path().display(),
    );

    let remote_base = format!(
        "git@{}:{}",
        rsync_host,
        if remote_path.ends_with('/') {
            remote_path.to_owned()
        } else {
            format!("{}/", remote_path)
        }
    );

    // (local_source, remote_destination)
    // .next/standalone contents go to the root of remote_path.
    // .next/static and public go into their correct subdirectories within it.
    let transfers: &[(&str, &str)] = &[
        // standalone contents → remote root (server.js lives here)
        (".next/standalone/", ""),
        // static chunks → .next/static/ inside the standalone tree
        (".next/static/", ".next/static/"),
        // public assets → public/ inside the standalone tree
        ("public/", "public/"),
    ];

    let mut upload_spinner = Spinner::new(
        Spinners::Hamburger,
        succeed_message(&format!("Uploading to {}…", remote_path)),
    );

    for (local_rel, remote_rel) in transfers {
        let local_path = format!("{}/{}", source, local_rel);
        let destination = format!("{}{}", remote_base, remote_rel);

        let output = Command::new("rsync")
            .args([
                "-az",
                "--delete",
                "-e",
                &ssh_command,
                &local_path,
                &destination,
            ])
            .output()
            .map_err(|e| anyhow!(fail_message(&format!("Failed to launch rsync: {}", e))))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            upload_spinner.stop_and_persist(&fail_symbol(), fail_message("Upload failed."));
            mark_failed(
                &deploy_ref,
                &created_deployment,
                &config,
                env,
                &access_token,
            )
            .await;
            return Err(anyhow!(fail_message(&format!(
                "rsync of '{}' failed (status {}): {}",
                local_rel,
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ))));
        }
    }

    // known_hosts_file must stay alive until all rsync calls complete.
    drop(known_hosts_file);

    upload_spinner.stop_and_persist(&succeed_symbol(), succeed_message("Upload complete."));

    // ── Step 8: SSH remote restart ───────────────────────────────────────────
    //
    // Runs `node server.js` via pm2 inside the deployed standalone directory.
    // PORT and HOSTNAME are set so Next.js binds correctly behind nginx.
    //
    // We always delete the existing pm2 process (if any) and start fresh with
    // `node server.js`. A bare `pm2 restart` would re-execute the *old* command
    // (e.g. `next start --port XXXX` from a previous git-push deploy), which
    // fails when the working directory now contains standalone output instead of
    // the full Next.js build tree. Deleting first guarantees the entry point
    // and environment are always correct.
    //
    // The port defaults to 3000 and can be overridden with `port = XXXX` in
    // .smb/config.toml — it must match the nginx upstream configuration.

    let port = config.project.port.unwrap_or(3000);

    let deploy_script = format!(
        r#"set -e
APP_PATH="{remote_path}"
PM2_APP="{pm2_app}"

if [ ! -d "$APP_PATH" ]; then
    echo "Error: $APP_PATH is not a directory."
    exit 1
fi

cd "$APP_PATH"

echo "Starting $PM2_APP with pm2..."
if pm2 describe "$PM2_APP" > /dev/null 2>&1; then
    pm2 delete "$PM2_APP"
fi
PORT={port} HOSTNAME=127.0.0.1 pm2 start node --name "$PM2_APP" -- server.js
pm2 save
echo "Done."
"#,
        remote_path = remote_path,
        pm2_app = pm2_app,
        port = port,
    );

    let mut restart_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        succeed_message(&format!("Restarting {} on server…", pm2_app)),
    );

    // Fresh known_hosts file for the SSH exec (previous one was dropped).
    let mut ssh_known_hosts_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(
        ssh_known_hosts_file,
        "{}",
        known_hosts::for_host(&rsync_host)
    )
    .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    let mut child = Command::new("ssh")
        .args([
            "-i",
            &identity_file_str,
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            &format!(
                "UserKnownHostsFile={}",
                ssh_known_hosts_file.path().display()
            ),
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "PasswordAuthentication=no",
            "-o",
            "BatchMode=yes",
            &format!("git@{}", rsync_host),
            "bash -s",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!(fail_message(&format!("Failed to spawn SSH: {}", e))))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(deploy_script.as_bytes())
            .map_err(|e| anyhow!("Failed to write deploy script to SSH stdin: {}", e))?;
    }

    // stdin was dropped at the end of the if-let block above, sending EOF to
    // remote bash. wait_with_output() reads stdout/stderr then waits for exit.
    let ssh_output = child
        .wait_with_output()
        .map_err(|e| anyhow!("Failed to wait for SSH process: {}", e))?;

    drop(ssh_known_hosts_file);

    if !ssh_output.status.success() {
        restart_spinner.stop_and_persist(&fail_symbol(), fail_message("Remote restart failed."));
        let stderr = String::from_utf8_lossy(&ssh_output.stderr);
        let stdout = String::from_utf8_lossy(&ssh_output.stdout);
        let details = if !stderr.trim().is_empty() {
            stderr
        } else {
            stdout
        };
        if !details.trim().is_empty() {
            eprintln!("{}", details.trim());
        }
        mark_failed(
            &deploy_ref,
            &created_deployment,
            &config,
            env,
            &access_token,
        )
        .await;
        return Err(anyhow!(fail_message(&format!(
            "SSH deploy script exited with status {}",
            ssh_output.status
        ))));
    }

    restart_spinner.stop_and_persist(
        &succeed_symbol(),
        succeed_message(&format!("{} restarted.", pm2_app)),
    );

    // ── Step 9: mark deployment as Done ──────────────────────────────────────

    if let Some(ref deployment) = created_deployment {
        match update(
            env,
            client(),
            access_token,
            config.project.id,
            deployment.id,
            DeploymentPayload {
                commit_hash: deploy_ref,
                status: DeploymentStatus::Done,
            },
        )
        .await
        {
            Ok(_) => println!("App is running {}", succeed_symbol()),
            Err(e) => eprintln!("Error updating deployment status to Done: {}", e),
        }
    }

    Ok(CommandResult {
        spinner: Spinner::new(Spinners::Hamburger, String::new()),
        symbol: succeed_symbol(),
        msg: succeed_message("Deployment complete."),
    })
}

/// Mark the in-flight deployment record as Failed.
/// Called on any early-return error path so the dashboard reflects reality.
async fn mark_failed(
    deploy_ref: &str,
    created_deployment: &Option<smbcloud_model::project::Deployment>,
    config: &Config,
    env: Environment,
    access_token: &str,
) {
    if let Some(ref deployment) = created_deployment {
        let _ = update(
            env,
            client(),
            access_token.to_owned(),
            config.project.id,
            deployment.id,
            DeploymentPayload {
                commit_hash: deploy_ref.to_owned(),
                status: DeploymentStatus::Failed,
            },
        )
        .await;
    }
}
