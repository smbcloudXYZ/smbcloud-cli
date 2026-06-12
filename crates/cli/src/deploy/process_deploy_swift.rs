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
};

/// Default Swift SDK used to cross-compile for Linux. The Static Linux SDK
/// produces a fully static musl binary that runs on any Linux host.
const DEFAULT_SWIFT_SDK: &str = "x86_64-swift-linux-musl";

/// Deploys a Swift/Vapor app by cross-compiling a Linux binary natively on the
/// host with the Swift Static Linux SDK (no Docker, no emulation), uploading the
/// binary and resource directories via rsync, then restarting the process on the
/// server over SSH.
///
/// Required config fields:
///   - `kind = "swift"`
///   - `path`   — remote app directory on the server (relative to git home)
///   - `port`   — port Vapor binds to (must match nginx upstream)
///
/// Optional config fields:
///   - `source`          — local source directory (defaults to `.`)
///   - `binary_name`     — executable name; falls back to Package.swift product name
///   - `swift_sdk`       — Swift SDK id (defaults to `x86_64-swift-linux-musl`)
///   - `swift_toolchain` — `TOOLCHAINS` value for the build; needed on macOS
///     where the default `swift` is Apple's Xcode toolchain (no lld)
pub async fn process_deploy_swift(env: Environment, config: Config) -> Result<CommandResult> {
    let deploy_start = std::time::Instant::now();

    let source = config.project.source.as_deref().unwrap_or(".");
    let remote_path = config.project.path.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "path not set in .smb/config.toml (e.g. path = \"apps/swiftyidwebsite\")"
        ))
    })?;
    let port = config.project.port.ok_or_else(|| {
        anyhow!(fail_message(
            "port not set in .smb/config.toml (e.g. port = 3010)"
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
    let swift_sdk = config
        .project
        .swift_sdk
        .as_deref()
        .unwrap_or(DEFAULT_SWIFT_SDK);
    let swift_toolchain = config.project.swift_toolchain.as_deref();

    println!();
    println!("  {}", console::style(&config.name).white().bold());
    println!();

    let binary_path = build_linux_binary(source, &binary_name, swift_sdk, swift_toolchain)?;

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
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
        .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    // ── Prepare remote directory ─────────────────────────────────────────────

    let mut prepare_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        format!(
            "  {} {}",
            console::style("◼").cyan(),
            console::style("Preparing server…").dim()
        ),
    );

    let prepare_output = run_remote_script(
        &identity_file_str,
        &known_hosts_file,
        &rsync_host,
        &build_remote_prepare_script(remote_path),
    )
    .map_err(|e| {
        anyhow!(fail_message(&format!(
            "Failed to prepare remote directory: {}",
            e
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

    let binary_size = fs::metadata(&binary_path).map(|m| m.len()).unwrap_or(0);
    prepare_spinner.stop_and_persist(
        &format!("  {}", console::style("◼").cyan()),
        format!(
            "{}    {} → {} ({})",
            console::style("Upload").white().bold(),
            console::style(&binary_name).dim(),
            console::style(format!("{}:{}", &rsync_host, remote_path)).dim(),
            console::style(format_size(binary_size)).dim(),
        ),
    );

    // ── Upload binary + Resources/ + Public/ ─────────────────────────────────

    let mut upload_spinner = Spinner::new(
        Spinners::Hamburger,
        format!(
            "  {} {}",
            console::style("◼").cyan(),
            console::style("Uploading…").dim()
        ),
    );

    let ssh_command = build_ssh_command(&identity_file_str, &known_hosts_file);
    let remote_with_slash = ensure_trailing_slash(remote_path);
    let destination = format!("git@{}:{}", rsync_host, remote_with_slash);
    let binary_path_str = binary_path.to_string_lossy().into_owned();

    let binary_upload = Command::new("rsync")
        .args(["-az", "-e", &ssh_command, &binary_path_str, &destination])
        .output()
        .map_err(|e| anyhow!(fail_message(&format!("Failed to launch rsync: {}", e))))?;

    if !binary_upload.status.success() {
        upload_spinner.stop_and_persist(
            &fail_symbol(),
            format!(
                "  {} {}",
                console::style("✘").red(),
                fail_message("Binary upload failed")
            ),
        );
        print_output_details(&binary_upload);
        drop(known_hosts_file);
        mark_failed(
            &deploy_ref,
            &created_deployment,
            &config,
            env,
            &access_token,
        )
        .await;
        return Err(anyhow!(fail_message("Failed to upload Swift binary")));
    }

    // Resources/ — required by Leaf for template rendering; cwd of the running
    // binary must be the remote_path so Leaf resolves Resources/Views/ correctly.
    let resources_dir = source_dir.join("Resources");
    if resources_dir.exists() {
        let resources_upload = Command::new("rsync")
            .args([
                "-az",
                "--delete",
                "-e",
                &ssh_command,
                &format!("{}/Resources/", source),
                &format!("{}Resources/", destination),
            ])
            .output()
            .map_err(|e| anyhow!(fail_message(&format!("Failed to launch rsync: {}", e))))?;

        if !resources_upload.status.success() {
            upload_spinner.stop_and_persist(
                &fail_symbol(),
                format!(
                    "  {} {}",
                    console::style("✘").red(),
                    fail_message("Resources/ upload failed")
                ),
            );
            print_output_details(&resources_upload);
            drop(known_hosts_file);
            mark_failed(
                &deploy_ref,
                &created_deployment,
                &config,
                env,
                &access_token,
            )
            .await;
            return Err(anyhow!(fail_message("Failed to upload Resources/")));
        }
    }

    // Public/ — static assets served by Vapor's FileMiddleware.
    let public_dir = source_dir.join("Public");
    if public_dir.exists() {
        let public_upload = Command::new("rsync")
            .args([
                "-az",
                "--delete",
                "-e",
                &ssh_command,
                &format!("{}/Public/", source),
                &format!("{}Public/", destination),
            ])
            .output()
            .map_err(|e| anyhow!(fail_message(&format!("Failed to launch rsync: {}", e))))?;

        if !public_upload.status.success() {
            upload_spinner.stop_and_persist(
                &fail_symbol(),
                format!(
                    "  {} {}",
                    console::style("✘").red(),
                    fail_message("Public/ upload failed")
                ),
            );
            print_output_details(&public_upload);
            drop(known_hosts_file);
            mark_failed(
                &deploy_ref,
                &created_deployment,
                &config,
                env,
                &access_token,
            )
            .await;
            return Err(anyhow!(fail_message("Failed to upload Public/")));
        }
    }

    drop(known_hosts_file);
    upload_spinner.stop_and_persist(
        &format!("  {}", console::style("◼").cyan()),
        format!(
            "{}    Starting {}…",
            console::style("Launch").white().bold(),
            console::style(&binary_name).dim(),
        ),
    );

    // ── SSH remote start ─────────────────────────────────────────────────────

    let mut ssh_known_hosts_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(
        ssh_known_hosts_file,
        "{}",
        known_hosts::for_host(&rsync_host)
    )
    .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    let ssh_output = run_remote_script(
        &identity_file_str,
        &ssh_known_hosts_file,
        &rsync_host,
        &build_remote_start_script(remote_path, &binary_name, port),
    )
    .map_err(|e| anyhow!(fail_message(&format!("Failed to spawn SSH: {}", e))))?;

    drop(ssh_known_hosts_file);

    if !ssh_output.status.success() {
        println!(
            "  {} {}",
            console::style("✘").red(),
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
        console::style("◼").cyan(),
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

    let package_swift = source_dir.join("Package.swift");
    if package_swift.exists() {
        let contents = fs::read_to_string(&package_swift).map_err(|e| {
            anyhow!(fail_message(&format!(
                "Failed to read '{}': {}",
                package_swift.display(),
                e
            )))
        })?;
        if let Some(name) = parse_swift_package_name(&contents) {
            return Ok(name);
        }
    }

    // Last resort: use the project config name.
    let project_name = config.project.name.trim();
    if !project_name.is_empty() {
        return Ok(project_name.to_owned());
    }

    Err(anyhow!(fail_message(
        "Could not determine Swift binary name. Set 'binary_name' in .smb/config.toml."
    )))
}

/// Extracts the package name from the `Package(name: "X", ...)` call in Package.swift.
fn parse_swift_package_name(contents: &str) -> Option<String> {
    let package_idx = contents.find("Package(")?;
    let after_package = &contents[package_idx..];
    let name_idx = after_package.find("name:")?;
    let after_name = &after_package[name_idx + 5..];
    let quote_start = after_name.find('"')? + 1;
    let after_quote = &after_name[quote_start..];
    let quote_end = after_quote.find('"')?;
    let name = after_quote[..quote_end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

/// Cross-compiles the Swift app for Linux on the host using the Swift Static
/// Linux SDK (`swift build --swift-sdk <id>`). This runs natively — no Docker,
/// no QEMU emulation — and yields a fully static musl binary that runs on any
/// Linux host, so the server needs no Swift runtime installed.
///
/// On macOS the default `swift` is often Apple's Xcode toolchain, which lacks
/// `lld` and fails to link the Linux binary. Set `swift_toolchain` (passed as
/// `TOOLCHAINS`) to a swift.org toolchain in that case. When `swift` is already
/// a swift.org toolchain (e.g. via swiftly on the server), no toolchain override
/// is needed.
fn build_linux_binary(
    source: &str,
    binary_name: &str,
    swift_sdk: &str,
    swift_toolchain: Option<&str>,
) -> Result<PathBuf> {
    if !command_exists("swift") {
        return Err(anyhow!(fail_message(
            "`swift` not found on PATH. Install a Swift toolchain (https://swift.org/install)."
        )));
    }

    println!(
        "  {} {}    {} → {}",
        console::style("◼").cyan(),
        console::style("Build").white().bold(),
        console::style(binary_name).dim(),
        console::style(format!("{} (static)", swift_sdk)).dim(),
    );

    // Resolve the exact bin directory the SDK build will write to, instead of
    // hardcoding `.build/<sdk>/release`. This also surfaces a missing SDK early
    // with the real toolchain error rather than a confusing "binary not found".
    let bin_path = swift_show_bin_path(source, swift_sdk, swift_toolchain)?;

    // `-Xlinker -s` strips symbols at link time (lld). For a statically-linked
    // Swift binary this roughly cuts the size by two-thirds (the static runtime
    // and Foundation carry large debug sections), which matters because the whole
    // binary is rsynced on every deploy.
    let mut command = Command::new("swift");
    command
        .args([
            "build",
            "-c",
            "release",
            "--swift-sdk",
            swift_sdk,
            "-Xlinker",
            "-s",
        ])
        .current_dir(source)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(toolchain) = swift_toolchain {
        command.env("TOOLCHAINS", toolchain);
    }

    let status = command.status().map_err(|e| {
        anyhow!(fail_message(&format!(
            "Failed to spawn `swift build`: {}",
            e
        )))
    })?;

    if !status.success() {
        return Err(anyhow!(fail_message(&format!(
            "Swift cross-compile failed (status {}). If the linker errored with \
             `-fuse-ld=lld` / `invalid linker name`, your default `swift` is Apple's \
             Xcode toolchain (no lld) — set `swift_toolchain` in .smb/config.toml to a \
             swift.org toolchain (e.g. swift_toolchain = \"swift\").",
            status
        ))));
    }

    let binary_path = bin_path.join(binary_name);
    if !binary_path.exists() {
        return Err(anyhow!(fail_message(&format!(
            "Built binary not found at '{}'. Verify the executable product name matches \
             'binary_name' in .smb/config.toml.",
            binary_path.display()
        ))));
    }

    let size = fs::metadata(&binary_path).map(|m| m.len()).unwrap_or(0);
    println!(
        "  {} {}    {} → {} ({})",
        console::style("◼").cyan(),
        console::style("Build").white().bold(),
        console::style(binary_name).dim(),
        console::style(swift_sdk).dim(),
        console::style(format_size(size)).dim(),
    );

    Ok(binary_path)
}

/// Asks SwiftPM for the release bin path for the given SDK. Doubles as an early
/// validation that the SDK is installed and the toolchain can target it.
fn swift_show_bin_path(
    source: &str,
    swift_sdk: &str,
    swift_toolchain: Option<&str>,
) -> Result<PathBuf> {
    let mut command = Command::new("swift");
    command
        .args([
            "build",
            "-c",
            "release",
            "--swift-sdk",
            swift_sdk,
            "--show-bin-path",
        ])
        .current_dir(source);
    if let Some(toolchain) = swift_toolchain {
        command.env("TOOLCHAINS", toolchain);
    }

    let output = command.output().map_err(|e| {
        anyhow!(fail_message(&format!(
            "Failed to spawn `swift build --show-bin-path`: {}",
            e
        )))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(fail_message(&format!(
            "Could not resolve the Swift SDK '{}'. Install the Static Linux SDK \
             (`swift sdk install <artifactbundle URL>`) and ensure it matches your \
             toolchain version.\n{}",
            swift_sdk,
            stderr.trim()
        ))));
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(PathBuf::from(path))
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

fn build_remote_start_script(remote_path: &str, binary_name: &str, port: u16) -> String {
    format!(
        r#"set -e
APP_PATH={remote_path}
BINARY={binary_name}
PORT={port}

case "$APP_PATH" in
    /*) ;;
    *) APP_PATH="$HOME/$APP_PATH" ;;
esac

if [ ! -d "$APP_PATH" ]; then
    echo "Error: $APP_PATH is not a directory."
    exit 1
fi

cd "$APP_PATH"

if [ ! -f "$BINARY" ]; then
    echo "Error: $BINARY does not exist in $APP_PATH."
    exit 1
fi

chmod +x "$BINARY"

PID=$(pidof "$BINARY" 2>/dev/null || true)
if [ -n "$PID" ]; then
    echo "Stopping $BINARY (PID $PID)..."
    kill "$PID" 2>/dev/null || true
    sleep 2
    if kill -0 "$PID" 2>/dev/null; then
        kill -9 "$PID" 2>/dev/null || true
    fi
fi

nohup "./$BINARY" serve --env production --hostname 127.0.0.1 --port "$PORT" >> "$APP_PATH/$BINARY.log" 2>&1 < /dev/null &
sleep 2

NEW_PID=$(pidof "$BINARY" 2>/dev/null || true)
if [ -z "$NEW_PID" ]; then
    echo "Error: failed to start $BINARY. Check $APP_PATH/$BINARY.log"
    exit 1
fi

echo "Started $BINARY as $NEW_PID"
echo "Done."
"#,
        remote_path = shell_single_quote(remote_path),
        binary_name = shell_single_quote(binary_name),
        port = port,
    )
}

fn build_ssh_command(identity_file: &str, known_hosts_file: &NamedTempFile) -> String {
    format!(
        "ssh -i {} \
         -o StrictHostKeyChecking=yes \
         -o UserKnownHostsFile={} \
         -o IdentitiesOnly=yes \
         -o PasswordAuthentication=no \
         -o BatchMode=yes",
        identity_file,
        known_hosts_file.path().display(),
    )
}

fn run_remote_script(
    identity_file: &str,
    known_hosts_file: &NamedTempFile,
    rsync_host: &str,
    script: &str,
) -> Result<Output> {
    let mut child = Command::new("ssh")
        .args([
            "-i",
            identity_file,
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            &format!("UserKnownHostsFile={}", known_hosts_file.path().display()),
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "PasswordAuthentication=no",
            "-o",
            "BatchMode=yes",
            &format!("git@{}", rsync_host),
            "bash",
            "-s",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn SSH: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| anyhow!("Failed to write deploy script to SSH stdin: {}", e))?;
    }

    child
        .wait_with_output()
        .map_err(|e| anyhow!("Failed to wait for SSH process: {}", e))
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

fn ensure_trailing_slash(path: &str) -> String {
    if path.ends_with('/') {
        path.to_owned()
    } else {
        format!("{}/", path)
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{} MB", bytes / 1_000_000)
    } else {
        format!("{} KB", bytes / 1_000)
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

    #[test]
    fn parses_package_name_from_package_swift() {
        let contents = r#"
// swift-tools-version:6.1.0
import PackageDescription

let package = Package(
    name: "SwiftyWebsite",
    platforms: [
        .macOS(.v13)
    ],
"#;
        assert_eq!(
            parse_swift_package_name(contents),
            Some("SwiftyWebsite".to_owned())
        );
    }

    #[test]
    fn parses_package_name_inline() {
        let contents = r#"let package = Package(name: "MyApp", dependencies: [])"#;
        assert_eq!(parse_swift_package_name(contents), Some("MyApp".to_owned()));
    }

    #[test]
    fn returns_none_for_missing_package_call() {
        assert_eq!(parse_swift_package_name("// no Package here"), None);
    }

    #[test]
    fn ensure_trailing_slash_adds_when_missing() {
        assert_eq!(ensure_trailing_slash("apps/web/foo"), "apps/web/foo/");
    }

    #[test]
    fn ensure_trailing_slash_noop_when_present() {
        assert_eq!(ensure_trailing_slash("apps/web/foo/"), "apps/web/foo/");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(45_000_000), "45 MB");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(512_000), "512 KB");
    }
}
