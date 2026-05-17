use {
    crate::{
        cli::CommandResult,
        client,
        deploy::known_hosts,
        ui::{fail_message, fail_symbol, succeed_symbol},
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
        fs,
        io::Write,
        path::{Path, PathBuf},
        process::{Command, Output, Stdio},
    },
    tempfile::NamedTempFile,
    toml::Value,
};

const DEFAULT_RUST_TARGET: &str = "x86_64-unknown-linux-gnu";

/// Deploys a Rust service by cross-compiling a Linux binary locally, uploading
/// only the executable, and restarting it on the remote host.
///
/// Required config fields:
///   - `kind = "rust"`
///   - `path`        — remote app directory on the server
///
/// Optional config fields:
///   - `source`      — local crate directory (defaults to current directory)
///   - `binary_name` — binary filename to upload; falls back to Cargo package name
///   - `rust_target` — local cross-compilation target triple; defaults to `x86_64-unknown-linux-gnu`
pub async fn process_deploy_rust(env: Environment, config: Config) -> Result<CommandResult> {
    let deploy_start = std::time::Instant::now();

    let source = config.project.source.as_deref().unwrap_or(".");
    let remote_path = config.project.path.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "path not set in .smb/config.toml (e.g. path = \"apps/rest-api/my-rust-app\")"
        ))
    })?;

    let source_dir = Path::new(source);
    if !source_dir.exists() {
        return Err(anyhow!(fail_message(&format!(
            "Source path '{}' does not exist. Check the 'source' field in .smb/config.toml.",
            source
        ))));
    }

    let binary_name = resolve_binary_name(&config, source_dir)?;
    let rust_target = config
        .project
        .rust_target
        .as_deref()
        .unwrap_or(DEFAULT_RUST_TARGET);

    // Header
    println!();
    println!("  {}", console::style(&config.name).white().bold());
    println!();

    let binary_path = build_local_binary(source, rust_target, &binary_name)?;

    let access_token = crate::token::get_smb_token::get_smb_token(env)?;
    let user = me(env, client(), &access_token).await?;
    let runner = config.project.runner;
    let rsync_host = runner.rsync_host();

    let deploy_ref = git2::Repository::discover(source)
        .ok()
        .and_then(|repo| {
            let head = repo.head().ok()?;
            let commit = head.peel_to_commit().ok()?;
            Some(commit.id().to_string())
        })
        .unwrap_or_else(|| Utc::now().format("%Y%m%dT%H%M%SZ").to_string());
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

    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let identity_file = home.join(".ssh").join(format!("id_{}@smbcloud", user.id));
    let identity_file_str = identity_file.to_string_lossy().into_owned();

    let mut known_hosts_file = NamedTempFile::new()
        .map_err(|error| anyhow!("Failed to create temp known_hosts file: {}", error))?;
    writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
        .map_err(|error| anyhow!("Failed to write known_hosts: {}", error))?;

    let mut prepare_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        format!(
            "  {} {}",
            console::style("◼").cyan(),
            console::style("Preparing server…").dim()
        ),
    );

    let prepare_script = build_remote_prepare_script(remote_path);
    let prepare_output = run_remote_script(
        &identity_file_str,
        &known_hosts_file,
        &rsync_host,
        &prepare_script,
    )
    .map_err(|error| {
        anyhow!(fail_message(&format!(
            "Failed to prepare remote directory: {}",
            error
        )))
    })?;

    if !prepare_output.status.success() {
        prepare_spinner.stop_and_persist(
            &fail_symbol(),
            format!(
                "  {} {}",
                console::style("✘").red(),
                fail_message("Server prepare failed")
            ),
        );
        print_output_details(&prepare_output);
        drop(known_hosts_file);
        mark_failed(
            &deploy_ref,
            &created_deployment,
            &config,
            env,
            &access_token,
        )
        .await;
        return Err(anyhow!(fail_message(&format!(
            "Failed to prepare remote directory '{}'",
            remote_path
        ))));
    }

    let ssh_command = build_ssh_command(&identity_file_str, &known_hosts_file);
    let remote_with_slash = if remote_path.ends_with('/') {
        remote_path.to_owned()
    } else {
        format!("{}/", remote_path)
    };
    let destination = format!("git@{}:{}", rsync_host, remote_with_slash);
    let binary_path_str = binary_path.to_string_lossy().into_owned();

    let binary_size = fs::metadata(&binary_path).map(|m| m.len()).unwrap_or(0);
    let upload_size = if binary_size > 1_000_000 {
        format!("{} MB", binary_size / 1_000_000)
    } else {
        format!("{} KB", binary_size / 1_000)
    };

    prepare_spinner.stop_and_persist(
        &format!("  {}", console::style("\u{25fc}").cyan()),
        format!(
            "{}    {} \u{2192} {} ({})",
            console::style("Upload").white().bold(),
            console::style(&binary_name).dim(),
            console::style(format!("{}:{}", &rsync_host, remote_with_slash)).dim(),
            console::style(&upload_size).dim(),
        ),
    );

    let mut upload_spinner = Spinner::new(
        Spinners::Hamburger,
        format!(
            "  {} {}",
            console::style("\u{25fc}").cyan(),
            console::style("Uploading\u{2026}").dim()
        ),
    );

    let upload_output = Command::new("rsync")
        .args(["-az", "-e", &ssh_command, &binary_path_str, &destination])
        .output()
        .map_err(|error| anyhow!(fail_message(&format!("Failed to launch rsync: {}", error))))?;

    if !upload_output.status.success() {
        upload_spinner.stop_and_persist(
            &fail_symbol(),
            format!(
                "  {} {}",
                console::style("✘").red(),
                fail_message("Upload failed")
            ),
        );
        print_output_details(&upload_output);
        drop(known_hosts_file);
        mark_failed(
            &deploy_ref,
            &created_deployment,
            &config,
            env,
            &access_token,
        )
        .await;
        return Err(anyhow!(fail_message(&format!(
            "Failed to upload '{}' to '{}'",
            binary_name, remote_path
        ))));
    }

    upload_spinner.stop_and_persist(
        &format!("  {}", console::style("\u{25fc}").cyan()),
        format!(
            "{}    Starting {}\u{2026}",
            console::style("Launch").white().bold(),
            console::style(&binary_name).dim(),
        ),
    );

    let deploy_script = build_remote_start_script(remote_path, &binary_name);
    let ssh_output = run_remote_script(
        &identity_file_str,
        &known_hosts_file,
        &rsync_host,
        &deploy_script,
    )
    .map_err(|error| anyhow!(fail_message(&format!("Failed to spawn SSH: {}", error))))?;

    drop(known_hosts_file);

    if !ssh_output.status.success() {
        println!(
            "  {} {}",
            console::style("\u{2718}").red(),
            fail_message("Launch failed")
        );
        print_output_details(&ssh_output);
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

    let stdout_text = String::from_utf8_lossy(&ssh_output.stdout);
    let pid_line = stdout_text
        .lines()
        .find(|line| line.starts_with("Started"))
        .unwrap_or("Started");

    println!(
        "  {} {}    {}",
        console::style("\u{25fc}").cyan(),
        console::style("Launch").white().bold(),
        console::style(pid_line).dim(),
    );

    if let Some(ref deployment) = created_deployment {
        let _ = update(
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
        .await;
    }

    let elapsed = deploy_start.elapsed().as_secs();
    let duration = if elapsed >= 60 {
        format!("{}m {}s", elapsed / 60, elapsed % 60)
    } else {
        format!("{}s", elapsed)
    };

    println!();

    Ok(CommandResult {
        spinner: Spinner::new(Spinners::Hamburger, String::new()),
        symbol: succeed_symbol(),
        msg: format!(
            "Deployed {} in {}",
            console::style(&config.name).white().bold(),
            console::style(&duration).cyan(),
        ),
    })
}

fn resolve_binary_name(config: &Config, source_dir: &Path) -> Result<String> {
    if let Some(binary_name) = config.project.binary_name.as_deref() {
        let binary_name = binary_name.trim();
        if binary_name.is_empty() {
            return Err(anyhow!(fail_message(
                "binary_name in .smb/config.toml cannot be empty."
            )));
        }
        return Ok(binary_name.to_owned());
    }

    let cargo_toml_path = source_dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(anyhow!(fail_message(&format!(
            "Cargo.toml not found at '{}'. Set 'source' to the crate directory or add 'binary_name' to .smb/config.toml.",
            cargo_toml_path.display()
        ))));
    }

    let cargo_toml = fs::read_to_string(&cargo_toml_path).map_err(|error| {
        anyhow!(fail_message(&format!(
            "Failed to read '{}': {}",
            cargo_toml_path.display(),
            error
        )))
    })?;

    let manifest: Value = toml::from_str(&cargo_toml).map_err(|error| {
        anyhow!(fail_message(&format!(
            "Failed to parse '{}': {}",
            cargo_toml_path.display(),
            error
        )))
    })?;

    let package_name = manifest
        .get("package")
        .and_then(Value::as_table)
        .and_then(|package| package.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            anyhow!(fail_message(
                "Could not determine Rust binary name from Cargo.toml. Add 'binary_name' to .smb/config.toml."
            ))
        })?;

    Ok(package_name.to_owned())
}

fn build_local_binary(source: &str, rust_target: &str, binary_name: &str) -> Result<PathBuf> {
    let mut build_command;
    let build_tool;
    let is_native = native_linux_target() == Some(rust_target);

    if !is_native && command_exists("cargo-zigbuild") {
        build_tool = "cargo zigbuild";
        build_command = Command::new("cargo");
        build_command.args([
            "zigbuild",
            "--release",
            "--target",
            rust_target,
            "--bin",
            binary_name,
        ]);
    } else if !is_native && command_exists("cross") {
        build_tool = "cross";
        build_command = Command::new("cross");
        build_command.args([
            "build",
            "--release",
            "--target",
            rust_target,
            "--bin",
            binary_name,
        ]);
    } else if is_native {
        build_tool = "cargo";
        build_command = Command::new("cargo");
        build_command.args([
            "build",
            "--release",
            "--target",
            rust_target,
            "--bin",
            binary_name,
        ]);
    } else {
        return Err(anyhow!(fail_message(&format!(
            "Cross-compilation tooling is required to build target '{}'. Install `cargo-zigbuild` (recommended) or `cross`, or run deploy from a matching Linux host.",
            rust_target
        ))));
    }

    let status = build_command
        .current_dir(source)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| {
            anyhow!(fail_message(&format!(
                "Failed to spawn '{} build' for target '{}': {}",
                build_tool, rust_target, error
            )))
        })?;

    if !status.success() {
        return Err(anyhow!(fail_message(&format!(
            "'{} build --release --target {} --bin {}' exited with status {}",
            build_tool, rust_target, binary_name, status
        ))));
    }

    let binary_path = Path::new(source)
        .join("target")
        .join(rust_target)
        .join("release")
        .join(binary_name);

    if !binary_path.exists() {
        return Err(anyhow!(fail_message(&format!(
            "Built binary not found at '{}'.",
            binary_path.display()
        ))));
    }

    let binary_size = fs::metadata(&binary_path).map(|m| m.len()).unwrap_or(0);
    let size_display = if binary_size > 1_000_000 {
        format!("{} MB", binary_size / 1_000_000)
    } else {
        format!("{} KB", binary_size / 1_000)
    };

    println!(
        "  {} {}    {} \u{2192} {} ({})",
        console::style("\u{25fc}").cyan(),
        console::style("Build").white().bold(),
        console::style(binary_name).dim(),
        console::style(rust_target).dim(),
        console::style(&size_display).dim(),
    );

    Ok(binary_path)
}

fn build_remote_prepare_script(remote_path: &str) -> String {
    format!(
        r#"set -e
APP_PATH={remote_path}

case "$APP_PATH" in
    /*) ;;
    *) APP_PATH="$HOME/$APP_PATH" ;;
esac

mkdir -p "$APP_PATH"
echo "Prepared $APP_PATH"
"#,
        remote_path = shell_single_quote(remote_path),
    )
}

fn build_remote_start_script(remote_path: &str, binary_name: &str) -> String {
    format!(
        r#"set -e
APP_PATH={remote_path}
PROCESS_NAME={binary_name}

case "$APP_PATH" in
    /*) ;;
    *) APP_PATH="$HOME/$APP_PATH" ;;
esac

if [ ! -d "$APP_PATH" ]; then
    echo "Error: $APP_PATH is not a directory."
    exit 1
fi

cd "$APP_PATH"

if [ ! -f "$PROCESS_NAME" ]; then
    echo "Error: $PROCESS_NAME does not exist in $APP_PATH."
    exit 1
fi

chmod +x "$PROCESS_NAME"

PID=$(pidof "$PROCESS_NAME" 2>/dev/null || true)

if [ -n "$PID" ]; then
    echo "Stopping $PROCESS_NAME ($PID)..."
    kill "$PID" 2>/dev/null || true
    sleep 2
    if kill -0 "$PID" 2>/dev/null; then
        echo "Force-killing $PROCESS_NAME ($PID)..."
        kill -9 "$PID" 2>/dev/null || true
    fi
fi

echo "Starting $PROCESS_NAME..."
nohup "./$PROCESS_NAME" >> "$APP_PATH/$PROCESS_NAME.log" 2>&1 &
sleep 1

NEW_PID=$(pidof "$PROCESS_NAME" 2>/dev/null || true)
if [ -z "$NEW_PID" ]; then
    echo "Error: failed to start $PROCESS_NAME."
    exit 1
fi

echo "Started $PROCESS_NAME as $NEW_PID"
echo "Done."
"#,
        remote_path = shell_single_quote(remote_path),
        binary_name = shell_single_quote(binary_name),
    )
}

fn build_ssh_command(identity_file: &str, known_hosts_file: &NamedTempFile) -> String {
    format!(
        "ssh -i {} -o StrictHostKeyChecking=yes -o UserKnownHostsFile={} -o IdentitiesOnly=yes -o PasswordAuthentication=no -o BatchMode=yes",
        identity_file,
        known_hosts_file.path().display(),
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

fn run_remote_script(
    identity_file: &str,
    known_hosts_file: &NamedTempFile,
    rsync_host: &str,
    script: &str,
) -> Result<Output> {
    let mut child = Command::new("ssh")
        .args(build_ssh_args(identity_file, known_hosts_file, rsync_host))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| anyhow!("Failed to spawn SSH: {}", error))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(script.as_bytes())
            .map_err(|error| anyhow!("Failed to write deploy script to SSH stdin: {}", error))?;
    }

    child
        .wait_with_output()
        .map_err(|error| anyhow!("Failed to wait for SSH process: {}", error))
}

fn print_output_details(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let details = if !stderr.trim().is_empty() {
        stderr
    } else {
        stdout
    };

    if !details.trim().is_empty() {
        eprintln!("{}", details.trim());
    }
}

fn command_exists(command: &str) -> bool {
    Command::new(command).arg("--version").output().is_ok()
}

fn native_linux_target() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        _ => None,
    }
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
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
                frontend_app_id: config.project.frontend_app_id.clone(),
            },
        )
        .await;
    }
}
