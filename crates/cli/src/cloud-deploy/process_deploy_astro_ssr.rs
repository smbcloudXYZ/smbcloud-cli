use {
    super::{
        process_deploy_nextjs_ssr::{mark_failed, restart_stanza},
        resolve_rsync_host,
    },
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
        path::Path,
        process::{Command, Stdio},
    },
    tempfile::NamedTempFile,
};

/// How the server installs production dependencies for a package manager, and
/// the lockfile that install needs.
struct PackageManagerPlan {
    lockfile: &'static str,
    install_command: &'static str,
}

fn package_manager_plan(package_manager: &str) -> Result<PackageManagerPlan> {
    match package_manager {
        "pnpm" => Ok(PackageManagerPlan {
            lockfile: "pnpm-lock.yaml",
            install_command: "pnpm install --prod --frozen-lockfile",
        }),
        "npm" => Ok(PackageManagerPlan {
            lockfile: "package-lock.json",
            install_command: "npm ci --omit=dev",
        }),
        other => Err(anyhow!(fail_message(&format!(
            "Unsupported package_manager '{other}' for kind \"astro-ssr\". Expected \"pnpm\" or \"npm\"."
        )))),
    }
}

/// Shell script run on the server after the upload: install production
/// dependencies next to the uploaded `dist/`, then restart the app's
/// `systemctl --user` service.
///
/// The `@astrojs/node` standalone server leaves its runtime dependencies out of
/// `dist/server`, and a Linux server cannot reuse a macOS `node_modules`, so
/// the server installs them itself from the uploaded lockfile.
fn remote_script(
    remote_path: &str,
    service: &str,
    install_command: &str,
    port: u16,
) -> Result<String> {
    let restart = restart_stanza(Some("systemd"), port)?;
    Ok(format!(
        r#"set -e
source ~/.profile 2>/dev/null || true
source ~/.bashrc 2>/dev/null || true
APP_PATH="{remote_path}"
PM2_APP="{service}"

case "$APP_PATH" in
    /*) ;;
    *) APP_PATH="$HOME/$APP_PATH" ;;
esac

if [ ! -d "$APP_PATH" ]; then
    echo "Error: $APP_PATH is not a directory."
    exit 1
fi

cd "$APP_PATH"
echo "Installing production dependencies…"
{install_command}

{restart}"#
    ))
}

/// Deploys an Astro SSR app built with `@astrojs/node` in standalone mode.
///
/// Required config fields:
///   - `kind = "astro-ssr"`
///   - `path`    — remote app directory on the server
///   - `pm2_app` — name of the `systemctl --user` service (`<pm2_app>.service`),
///     created on the server beforehand. The field keeps its `nextjs-ssr` name.
///
/// Optional config fields:
///   - `source`          — local app directory (defaults to the current directory)
///   - `package_manager` — `"pnpm"` (default) or `"npm"`
///
/// The port is not a deploy setting: the service unit decides what the app binds.
///
/// Steps:
///   1. install and build locally (`dist/client` + `dist/server/entry.mjs`)
///   2. rsync `dist/`, `package.json`, the lockfile and `pnpm-workspace.yaml`
///   3. SSH: install production dependencies, restart the service
///
/// Environment variables (such as secrets) belong to the service on the
/// server, never to the upload.
pub async fn process_deploy_astro_ssr(env: Environment, config: Config) -> Result<CommandResult> {
    let source = config.project.source.as_deref().unwrap_or(".");
    let package_manager = config.project.package_manager.as_deref().unwrap_or("pnpm");
    let plan = package_manager_plan(package_manager)?;

    let remote_path = config.project.path.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "path not set in .smb/config.toml (e.g. path = \"apps/web/myapp\")"
        ))
    })?;

    let service = config.project.pm2_app.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "pm2_app not set in .smb/config.toml. For kind \"astro-ssr\" it names the systemd user service (e.g. pm2_app = \"my-app\")."
        ))
    })?;

    let source_dir = Path::new(source);
    if !source_dir.exists() {
        return Err(anyhow!(fail_message(&format!(
            "Source path '{}' does not exist. Check the 'source' field in .smb/config.toml.",
            source
        ))));
    }
    if !source_dir.join(plan.lockfile).exists() {
        return Err(anyhow!(fail_message(&format!(
            "{} not found in '{}'. The server installs dependencies from it.",
            plan.lockfile, source
        ))));
    }

    let access_token = crate::token::get_smb_token::get_smb_token(env)?;
    let user = me(env, client(), &access_token).await?;

    // ── Step 1: install and build locally ────────────────────────────────────

    let mut install_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        succeed_message(&format!("Installing dependencies in {}…", source)),
    );
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
        let details = if stderr.trim().is_empty() {
            stdout
        } else {
            stderr
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

    println!();
    let build_status = Command::new(package_manager)
        .args(["run", "build"])
        .current_dir(source)
        .status()
        .map_err(|e| {
            anyhow!(fail_message(&format!(
                "Failed to spawn '{} run build': {}",
                package_manager, e
            )))
        })?;
    println!();

    if !build_status.success() {
        return Err(anyhow!(fail_message(&format!(
            "'{} run build' exited with status {}",
            package_manager, build_status
        ))));
    }

    // A static build, or a missing adapter, succeeds but leaves no server.
    if !source_dir.join("dist/server/entry.mjs").exists() {
        return Err(anyhow!(fail_message(
            "dist/server/entry.mjs not found. Add the @astrojs/node adapter with mode: 'standalone' to astro.config."
        )));
    }
    println!(
        "{} {}",
        succeed_symbol(),
        succeed_message("Build complete.")
    );

    // ── Step 2: record the deployment, then upload ───────────────────────────

    let deploy_ref = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let created_deployment = create_deployment(
        env,
        client(),
        &access_token,
        config.project.id,
        DeploymentPayload {
            commit_hash: deploy_ref.clone(),
            status: DeploymentStatus::Started,
            frontend_app_id: config.project.frontend_app_id.clone(),
        },
    )
    .await
    .ok();

    let rsync_host =
        resolve_rsync_host(config.project.rsync_host.as_deref(), &config.project.runner)?;

    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let identity_file = home.join(".ssh").join(format!("id_{}@smbcloud", user.id));
    let identity_file_str = identity_file.to_string_lossy().into_owned();

    let mut known_hosts_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
        .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    let ssh_options = [
        "-i".to_string(),
        identity_file_str.clone(),
        "-o".to_string(),
        "StrictHostKeyChecking=yes".to_string(),
        "-o".to_string(),
        format!("UserKnownHostsFile={}", known_hosts_file.path().display()),
        "-o".to_string(),
        "IdentitiesOnly=yes".to_string(),
        "-o".to_string(),
        "PasswordAuthentication=no".to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
    ];
    let ssh_command = format!("ssh {}", ssh_options.join(" "));

    let remote_base = format!(
        "git@{}:{}",
        rsync_host,
        if remote_path.ends_with('/') {
            remote_path.to_owned()
        } else {
            format!("{}/", remote_path)
        }
    );

    // `dist/` mirrors exactly (--delete). The manifest files go to the app root
    // without --delete so the server's own files (service env, logs) survive.
    let manifest_files: Vec<String> = [plan.lockfile, "package.json", "pnpm-workspace.yaml"]
        .iter()
        .map(|name| format!("{}/{}", source.trim_end_matches('/'), name))
        .filter(|path| Path::new(path).exists())
        .collect();

    let mut upload_spinner = Spinner::new(
        Spinners::Hamburger,
        succeed_message(&format!("Uploading to {}…", remote_path)),
    );

    let uploads = [
        (
            vec![
                "-az".to_string(),
                "--delete".to_string(),
                "-e".to_string(),
                ssh_command.clone(),
                format!("{}/dist/", source.trim_end_matches('/')),
                format!("{}dist/", remote_base),
            ],
            "dist/",
        ),
        (
            [
                vec!["-az".to_string(), "-e".to_string(), ssh_command.clone()],
                manifest_files,
                vec![remote_base.clone()],
            ]
            .concat(),
            "package manifest",
        ),
    ];

    for (rsync_args, label) in uploads {
        let output = Command::new("rsync")
            .args(&rsync_args)
            .output()
            .map_err(|e| anyhow!(fail_message(&format!("Failed to launch rsync: {}", e))))?;
        if !output.status.success() {
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
                "rsync of {} failed (status {}): {}",
                label,
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).trim()
            ))));
        }
    }
    upload_spinner.stop_and_persist(&succeed_symbol(), succeed_message("Upload complete."));

    // ── Step 3: install runtime dependencies and restart ─────────────────────

    let script = remote_script(
        remote_path,
        service,
        plan.install_command,
        config.project.port.unwrap_or(3000),
    )?;

    let mut restart_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        succeed_message(&format!("Restarting {} on server…", service)),
    );

    let mut child = Command::new("ssh")
        .args(&ssh_options)
        .arg(format!("git@{}", rsync_host))
        .arg("bash -s")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!(fail_message(&format!("Failed to spawn SSH: {}", e))))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| anyhow!("Failed to write deploy script to SSH stdin: {}", e))?;
    }
    let ssh_output = child
        .wait_with_output()
        .map_err(|e| anyhow!("Failed to wait for SSH process: {}", e))?;

    // The known_hosts file must outlive every rsync and SSH call above.
    drop(known_hosts_file);

    if !ssh_output.status.success() {
        restart_spinner.stop_and_persist(&fail_symbol(), fail_message("Remote restart failed."));
        let stderr = String::from_utf8_lossy(&ssh_output.stderr);
        let stdout = String::from_utf8_lossy(&ssh_output.stdout);
        let details = if stderr.trim().is_empty() {
            stdout
        } else {
            stderr
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
        succeed_message(&format!("{} restarted.", service)),
    );

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
                frontend_app_id: config.project.frontend_app_id.clone(),
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

#[cfg(test)]
mod tests {
    use super::{package_manager_plan, remote_script};

    #[test]
    fn package_managers_map_to_a_lockfile_and_install() {
        let pnpm = package_manager_plan("pnpm").expect("pnpm is supported");
        assert_eq!(pnpm.lockfile, "pnpm-lock.yaml");
        assert!(pnpm.install_command.contains("--frozen-lockfile"));

        let npm = package_manager_plan("npm").expect("npm is supported");
        assert_eq!(npm.lockfile, "package-lock.json");
    }

    #[test]
    fn unknown_package_manager_is_rejected() {
        let message = package_manager_plan("yarn")
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(message.contains("yarn"), "names the value: {message}");
    }

    #[test]
    fn remote_script_installs_then_restarts_the_user_service() {
        let script = remote_script(
            "apps/web/example",
            "example-web",
            "pnpm install --prod --frozen-lockfile",
            3000,
        )
        .expect("script builds");

        let install = script.find("pnpm install --prod").expect("installs");
        let restart = script
            .find("systemctl --user restart")
            .expect("restarts via systemd");
        assert!(install < restart, "dependencies land before the restart");
        assert!(script.contains("PM2_APP=\"example-web\""));
        assert!(!script.contains("pm2 "), "no pm2 fallback");
        assert!(!script.contains("sudo"), "no sudo");
    }
}
