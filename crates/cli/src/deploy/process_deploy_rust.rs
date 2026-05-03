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
        path::Path,
        process::{Command, Stdio},
    },
    tempfile::NamedTempFile,
};

/// Deploys a Rust service by syncing source to the server and running a remote
/// Cargo build step.
///
/// Required config fields:
///   - `kind = "rust"`
///   - `path`   — remote app directory on the server
///
/// Optional config fields:
///   - `source`      — local source directory (defaults to current directory)
///   - `compile_cmd` — remote command sequence to run inside `path` after the
///     upload. When omitted, the CLI runs `cargo build --release`.
///
/// Typical `compile_cmd` for a systemd-managed service:
///   cargo build --release && sudo systemctl restart smbcloud-mail-imap
pub async fn process_deploy_rust(env: Environment, config: Config) -> Result<CommandResult> {
    let source = config.project.source.as_deref().unwrap_or(".");
    let remote_path = config.project.path.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "path not set in .smb/config.toml (e.g. path = \"apps/mail/smbcloud-mail-imap\")"
        ))
    })?;

    let source_dir = Path::new(source);
    if !source_dir.exists() {
        return Err(anyhow!(fail_message(&format!(
            "Source path '{}' does not exist. Check the 'source' field in .smb/config.toml.",
            source
        ))));
    }

    let access_token = crate::token::get_smb_token::get_smb_token(env)?;
    let user = me(env, client(), &access_token).await?;
    let runner = config.project.runner;
    let rsync_host = runner.rsync_host();

    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let identity_file = home.join(".ssh").join(format!("id_{}@smbcloud", user.id));
    let identity_file_str = identity_file.to_string_lossy().into_owned();

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

    let mut known_hosts_file = NamedTempFile::new()
        .map_err(|error| anyhow!("Failed to create temp known_hosts file: {}", error))?;
    writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
        .map_err(|error| anyhow!("Failed to write known_hosts: {}", error))?;

    let ssh_command = build_ssh_command(&identity_file_str, &known_hosts_file);

    let source_with_slash = if source.ends_with('/') {
        source.to_owned()
    } else {
        format!("{}/", source)
    };

    let remote_with_slash = if remote_path.ends_with('/') {
        remote_path.to_owned()
    } else {
        format!("{}/", remote_path)
    };

    let destination = format!("git@{}:{}", rsync_host, remote_with_slash);

    let mut upload_spinner = Spinner::new(
        Spinners::Hamburger,
        succeed_message(&format!("Uploading Rust project {}…", source)),
    );

    let upload_output = Command::new("rsync")
        .args([
            "-az",
            "--delete",
            "--exclude=.git",
            "--exclude=.smb",
            "--exclude=target",
            "-e",
            &ssh_command,
            &source_with_slash,
            &destination,
        ])
        .output()
        .map_err(|error| anyhow!(fail_message(&format!("Failed to launch rsync: {}", error))))?;

    if !upload_output.status.success() {
        drop(known_hosts_file);
        upload_spinner.stop_and_persist(&fail_symbol(), fail_message("Upload failed."));
        let stderr = String::from_utf8_lossy(&upload_output.stderr);
        mark_failed(
            &deploy_ref,
            &created_deployment,
            &config,
            env,
            &access_token,
        )
        .await;
        return Err(anyhow!(fail_message(&format!(
            "rsync exited with status {}: {}",
            upload_output.status.code().unwrap_or(-1),
            stderr.trim()
        ))));
    }

    upload_spinner.stop_and_persist(&succeed_symbol(), succeed_message("Upload complete."));

    let remote_script_body = match config.project.compile_cmd.as_deref() {
        Some(command) => command.to_owned(),
        None => "cargo build --release".to_owned(),
    };

    let deploy_script = format!(
        r#"set -e
source ~/.profile 2>/dev/null || true
source ~/.bashrc 2>/dev/null || true
APP_PATH=\"{remote_path}\"

if [ ! -d \"$APP_PATH\" ]; then
    echo \"Error: $APP_PATH is not a directory.\"
    exit 1
fi

cd \"$APP_PATH\"
{remote_script_body}
echo \"Done.\"
"#,
        remote_path = remote_path,
        remote_script_body = remote_script_body,
    );

    let mut remote_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        succeed_message("Running remote Rust deploy script…"),
    );

    let mut child = Command::new("ssh")
        .args(build_ssh_args(
            &identity_file_str,
            &known_hosts_file,
            &rsync_host,
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| anyhow!(fail_message(&format!("Failed to spawn SSH: {}", error))))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(deploy_script.as_bytes())
            .map_err(|error| anyhow!("Failed to write deploy script to SSH stdin: {}", error))?;
    }

    let ssh_output = child
        .wait_with_output()
        .map_err(|error| anyhow!("Failed to wait for SSH process: {}", error))?;

    drop(known_hosts_file);

    if !ssh_output.status.success() {
        remote_spinner.stop_and_persist(&fail_symbol(), fail_message("Remote deploy failed."));
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

    remote_spinner.stop_and_persist(
        &succeed_symbol(),
        succeed_message("Remote Rust deploy script completed."),
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
            },
        )
        .await
        {
            Ok(_) => println!("App is running {}", succeed_symbol()),
            Err(error) => eprintln!("Error updating deployment status to Done: {}", error),
        }
    }

    Ok(CommandResult {
        spinner: Spinner::new(Spinners::Hamburger, String::new()),
        symbol: succeed_symbol(),
        msg: succeed_message("Deployment complete."),
    })
}

fn build_ssh_command(identity_file: &str, known_hosts_file: &NamedTempFile) -> String {
    format!(
        "ssh -i {identity} \\
         -o StrictHostKeyChecking=yes \\
         -o UserKnownHostsFile={known_hosts} \\
         -o IdentitiesOnly=yes \\
         -o PasswordAuthentication=no \\
         -o BatchMode=yes",
        identity = identity_file,
        known_hosts = known_hosts_file.path().display(),
    )
}

fn build_ssh_args<'a>(
    identity_file: &'a str,
    known_hosts_file: &'a NamedTempFile,
    rsync_host: &'a str,
) -> Vec<String> {
    vec![
        "-i".to_owned(),
        identity_file.to_owned(),
        "-o".to_owned(),
        "StrictHostKeyChecking=yes".to_owned(),
        "-o".to_owned(),
        format!("UserKnownHostsFile={}", known_hosts_file.path().display()),
        "-o".to_owned(),
        "IdentitiesOnly=yes".to_owned(),
        "-o".to_owned(),
        "PasswordAuthentication=no".to_owned(),
        "-o".to_owned(),
        "BatchMode=yes".to_owned(),
        format!("git@{}", rsync_host),
        "bash".to_owned(),
        "-s".to_owned(),
    ]
}

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
