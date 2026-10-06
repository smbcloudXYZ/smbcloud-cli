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
///   - `process_manager` — `"nohup"` (default) or `"systemd"`; systemd restarts
///     `<binary_name>.service` with `systemctl --user` (see `resolve_supervisor`)
///   - `port`, `health_path` — when both are set, the deploy polls
///     `http://127.0.0.1:<port><health_path>` after the restart
///
/// The binary is uploaded as `<binary>.new`, the live one is kept as
/// `<binary>.previous`, and a failed check restores it.
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

    let supervisor = resolve_supervisor(config.project.process_manager.as_deref())?;
    let health_url =
        resolve_health_url(config.project.port, config.project.health_path.as_deref())?;
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
    let destination = format!(
        "git@{}:{}{}.new",
        rsync_host, remote_with_slash, binary_name
    );
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
            console::style(format!("{}:{}", rsync_host, remote_with_slash)).dim(),
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

    let deploy_script =
        build_remote_start_script(remote_path, &binary_name, supervisor, health_url.as_deref());
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
        if let Some(line) = String::from_utf8_lossy(&ssh_output.stdout)
            .lines()
            .find(|line| line.starts_with("Rolled back"))
        {
            eprintln!("{}", line);
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

/// How the service is stopped and started on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Supervisor {
    /// Kill the old process, start the new one with `nohup`.
    Nohup,
    /// `systemctl --user restart <binary_name>.service`.
    Systemd,
}

/// Maps `process_manager` to a [`Supervisor`] (matched case-insensitively).
/// `None`, `""` and `"nohup"` keep the original behaviour. Any other value is a
/// hard error: a typo like `system` must not start a nohup process next to a
/// systemd-owned one that already holds the port.
fn resolve_supervisor(process_manager: Option<&str>) -> Result<Supervisor> {
    match process_manager.map(str::trim) {
        None | Some("") => Ok(Supervisor::Nohup),
        Some(m) if m.eq_ignore_ascii_case("nohup") => Ok(Supervisor::Nohup),
        Some(m) if m.eq_ignore_ascii_case("systemd") => Ok(Supervisor::Systemd),
        Some(other) => Err(anyhow!(fail_message(&format!(
            "Unknown process_manager '{other}' in .smb/config.toml. \
             Expected \"nohup\" (default) or \"systemd\"."
        )))),
    }
}

/// Builds the URL the remote script polls after the restart. `None` unless both
/// `port` and `health_path` are set. The path ends up in a shell script, so it
/// is limited to URL-path characters, and every `%` must start a valid
/// percent-escape.
fn resolve_health_url(port: Option<u16>, health_path: Option<&str>) -> Result<Option<String>> {
    let Some(path) = health_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return Ok(None);
    };
    let bytes = path.as_bytes();
    let valid = path.starts_with('/')
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | '~' | '%'))
        && bytes.iter().enumerate().all(|(i, &b)| {
            b != b'%'
                || bytes
                    .get(i + 1..i + 3)
                    .is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit))
        });
    if !valid {
        return Err(anyhow!(fail_message(&format!(
            "Invalid health_path '{path}' in .smb/config.toml. \
             It must start with '/' and contain only letters, digits and / - _ . ~ %; \
             each % must be followed by two hex digits."
        ))));
    }
    Ok(port.map(|port| format!("http://127.0.0.1:{port}{path}")))
}

/// Remote script: swap the uploaded `<binary>.new` into place (keeping the live
/// one as `<binary>.previous`), restart, verify, and roll back on failure.
fn build_remote_start_script(
    remote_path: &str,
    binary_name: &str,
    supervisor: Supervisor,
    health_url: Option<&str>,
) -> String {
    let mut script = format!(
        r#"set -e
exec 2>&1
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

if [ ! -f "$PROCESS_NAME.new" ]; then
    echo "Error: $PROCESS_NAME.new does not exist in $APP_PATH."
    exit 1
fi

"#,
        remote_path = shell_single_quote(remote_path),
        binary_name = shell_single_quote(binary_name),
    );
    script.push_str(&restart_section(supervisor));
    script.push_str(&verify_section(supervisor, health_url));
    script.push_str(swap_section());
    script.push_str(
        r#"if restart_service && check_service; then
    started
    echo "Done."
    exit 0
fi

echo "Error: $PROCESS_NAME failed its check after the restart."
show_logs
if [ -f "$PROCESS_NAME.previous" ]; then
    echo "Rolling back to the previous binary..."
    cp -p "$PROCESS_NAME.previous" "$PROCESS_NAME.rollback"
    mv -f "$PROCESS_NAME.rollback" "$PROCESS_NAME"
    if restart_service && check_service; then
        echo "Rolled back to the previous $PROCESS_NAME, which is serving again."
    else
        echo "Rolled back to the previous $PROCESS_NAME, but it failed its check too."
    fi
else
    echo "No previous binary to roll back to."
fi
exit 1
"#,
    );
    script
}

/// Keeps the live binary as `.previous` and moves `.new` into place.
fn swap_section() -> &'static str {
    r#"if [ -f "$PROCESS_NAME" ]; then
    cp -p "$PROCESS_NAME" "$PROCESS_NAME.previous"
fi
chmod +x "$PROCESS_NAME.new"
mv -f "$PROCESS_NAME.new" "$PROCESS_NAME"

"#
}

/// Defines `restart_service`. The systemd variant also fails early, before the
/// swap, when the unit does not exist.
fn restart_section(supervisor: Supervisor) -> String {
    match supervisor {
        Supervisor::Nohup => r#"restart_service() {
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
}

"#
        .to_owned(),
        Supervisor::Systemd => r#"export XDG_RUNTIME_DIR="/run/user/$(id -u)"
UNIT="$PROCESS_NAME.service"
if ! systemctl --user cat "$UNIT" >/dev/null 2>&1; then
    echo "Error: systemd --user unit $UNIT not found on the server."
    echo "Create ~/.config/systemd/user/$UNIT and enable it (see the"
    echo "smbcloud-deploy-rust guide, 'Process manager: systemd --user'),"
    echo "or unset process_manager to use the default start. Then re-deploy."
    exit 1
fi

restart_service() {
    echo "Restarting $UNIT via systemd --user..."
    systemctl --user restart "$UNIT"
}

"#
        .to_owned(),
    }
}

/// Defines `check_service`, `started` and `show_logs`. The check polls the
/// health URL for up to 30 seconds when there is one, else looks for a live
/// process (`is-active` under systemd, `pidof` under nohup).
fn verify_section(supervisor: Supervisor, health_url: Option<&str>) -> String {
    let mut section = String::new();
    match health_url {
        Some(url) => {
            section.push_str(&format!("HEALTH_URL={}\n\n", shell_single_quote(url)));
            section.push_str(
                r#"check_service() {
    echo "Waiting for $HEALTH_URL..."
    i=0
    while [ "$i" -lt 30 ]; do
        if curl -fsS -o /dev/null --max-time 2 "$HEALTH_URL"; then
            return 0
        fi
        i=$((i + 1))
        sleep 1
    done
    return 1
}

"#,
            );
        }
        None => section.push_str(match supervisor {
            Supervisor::Systemd => {
                r#"check_service() {
    sleep 3
    systemctl --user is-active "$UNIT" >/dev/null 2>&1
}

"#
            }
            Supervisor::Nohup => {
                r#"check_service() {
    sleep 1
    [ -n "$(pidof "$PROCESS_NAME" 2>/dev/null || true)" ]
}

"#
            }
        }),
    }
    section.push_str(match supervisor {
        Supervisor::Systemd => {
            r#"started() {
    echo "Started $PROCESS_NAME via $UNIT"
}

show_logs() {
    journalctl --user -u "$UNIT" -n 20 --no-pager 2>&1 || true
}

"#
        }
        Supervisor::Nohup => {
            r#"started() {
    echo "Started $PROCESS_NAME as $(pidof "$PROCESS_NAME" 2>/dev/null || true)"
}

show_logs() {
    tail -n 20 "$APP_PATH/$PROCESS_NAME.log" 2>&1 || true
}

"#
        }
    });
    section
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

#[cfg(test)]
mod tests {
    use super::*;

    fn script(supervisor: Supervisor, health_url: Option<&str>) -> String {
        build_remote_start_script("apps/my-app", "my-app", supervisor, health_url)
    }

    #[test]
    fn resolve_supervisor_defaults_to_nohup() {
        for pm in [
            None,
            Some(""),
            Some("nohup"),
            Some("NoHup"),
            Some(" nohup "),
        ] {
            assert_eq!(resolve_supervisor(pm).unwrap(), Supervisor::Nohup, "{pm:?}");
        }
    }

    #[test]
    fn resolve_supervisor_systemd_is_case_insensitive() {
        for pm in ["systemd", "SystemD", " systemd "] {
            assert_eq!(resolve_supervisor(Some(pm)).unwrap(), Supervisor::Systemd);
        }
    }

    #[test]
    fn resolve_supervisor_rejects_unknown_value() {
        for pm in ["system", "pm2", "supervisord"] {
            let msg = resolve_supervisor(Some(pm)).unwrap_err().to_string();
            assert!(msg.contains(pm), "names the offending value: {msg}");
            assert!(msg.contains("nohup") && msg.contains("systemd"), "{msg}");
        }
    }

    #[test]
    fn systemd_script_restarts_the_user_unit() {
        let s = script(Supervisor::Systemd, None);
        assert!(s.contains("systemctl --user restart \"$UNIT\""));
        assert!(s.contains("UNIT=\"$PROCESS_NAME.service\""));
        assert!(s.contains("XDG_RUNTIME_DIR=\"/run/user/$(id -u)\""));
        assert!(s.contains("systemctl --user is-active"));
        assert!(!s.contains("nohup"));
        assert!(!s.contains("kill"));
        assert!(!s.contains("sudo"));
    }

    #[test]
    fn nohup_script_keeps_the_pidof_flow() {
        let s = script(Supervisor::Nohup, None);
        assert!(s.contains("pidof"));
        assert!(s.contains("nohup \"./$PROCESS_NAME\""));
        assert!(!s.contains("systemctl"));
    }

    #[test]
    fn both_scripts_back_up_and_swap_before_restarting() {
        for supervisor in [Supervisor::Nohup, Supervisor::Systemd] {
            let s = script(supervisor, None);
            let backup = s
                .find("cp -p \"$PROCESS_NAME\" \"$PROCESS_NAME.previous\"")
                .unwrap();
            let swap = s
                .find("mv -f \"$PROCESS_NAME.new\" \"$PROCESS_NAME\"")
                .unwrap();
            let restart = s.find("if restart_service && check_service").unwrap();
            assert!(backup < swap && swap < restart, "{supervisor:?}");
            assert!(s.contains("Rolled back"), "{supervisor:?}");
        }
    }

    #[test]
    fn health_url_controls_the_curl_poll() {
        for supervisor in [Supervisor::Nohup, Supervisor::Systemd] {
            let with = script(supervisor, Some("http://127.0.0.1:8080/health"));
            assert!(with.contains("curl -fsS"));
            assert!(with.contains("HEALTH_URL='http://127.0.0.1:8080/health'"));
            assert!(!script(supervisor, None).contains("curl"));
        }
    }

    #[test]
    fn health_url_needs_both_port_and_path() {
        let url = resolve_health_url(Some(8080), Some("/health")).unwrap();
        assert_eq!(url.as_deref(), Some("http://127.0.0.1:8080/health"));
        assert_eq!(resolve_health_url(None, Some("/health")).unwrap(), None);
        assert_eq!(resolve_health_url(Some(8080), None).unwrap(), None);
        assert_eq!(resolve_health_url(Some(8080), Some("")).unwrap(), None);
    }

    #[test]
    fn health_path_validation_rejects_unsafe_values() {
        for path in [
            "health", "/a b", "/x;rm", "/$(id)", "/a'b", "/foo%ZZ", "/foo%", "/foo%2",
        ] {
            assert!(
                resolve_health_url(Some(8080), Some(path)).is_err(),
                "{path}"
            );
        }
    }

    #[test]
    fn health_path_accepts_valid_percent_escapes() {
        let url = resolve_health_url(Some(8080), Some("/a%20b")).unwrap();
        assert_eq!(url.as_deref(), Some("http://127.0.0.1:8080/a%20b"));
    }

    #[test]
    fn binary_name_with_single_quote_is_quoted() {
        let s = build_remote_start_script("apps/x", "a'b", Supervisor::Nohup, None);
        assert!(s.contains("PROCESS_NAME='a'\"'\"'b'"));
    }

    #[test]
    fn every_script_variant_is_valid_bash() {
        for supervisor in [Supervisor::Nohup, Supervisor::Systemd] {
            for health in [None, Some("http://127.0.0.1:8080/health")] {
                let mut child = Command::new("bash")
                    .args(["-n", "-s"])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("bash is available");
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(script(supervisor, health).as_bytes())
                    .unwrap();
                let out = child.wait_with_output().unwrap();
                assert!(
                    out.status.success(),
                    "{supervisor:?} {health:?}: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
    }
}
