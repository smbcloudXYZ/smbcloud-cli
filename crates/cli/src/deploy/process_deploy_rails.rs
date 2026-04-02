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

/// Deploys a Ruby on Rails sub-project from a monorepo.
///
/// Replicates the GitHub Actions workflow:
///
///   1. rsync shared `lib/` directory to the server (e.g. Rust-based gems)
///   2. SSH: compile native gem extensions on the server
///   3. git init inside the sub-project directory, commit all files,
///      force-push to the server's bare repo (triggers post-receive deploy)
///   4. Record deployment status via the API
///
/// Config fields used:
///   - `source`       — local sub-project directory (e.g. "backend/musik88-web")
///   - `repository`   — bare repo name on the server (e.g. "musik88-production")
///   - `runner`       — must be `Ruby` (determines git host: api-1.smbcloud.xyz)
///   - `shared_lib`   — optional path to shared lib directory to rsync (e.g. "lib")
///   - `compile_cmd`  — optional SSH command to run after rsync (e.g. gem compilation)
pub async fn process_deploy_rails(env: Environment, config: Config) -> Result<CommandResult> {
    let source = config.project.source.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "source not set in .smb/config.toml (e.g. source = \"backend/my-rails-app\")"
        ))
    })?;

    let repository = config.project.repository.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "repository not set in .smb/config.toml (e.g. repository = \"my-app-production\")"
        ))
    })?;

    let source_dir = std::path::Path::new(source);
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

    // ── Step 1: rsync shared lib to the server ───────────────────────────────
    //
    // Some Rails apps depend on shared libraries in the monorepo root (e.g.
    // Rust-based gems that must be compiled on the server). If `shared_lib`
    // is set, rsync that directory to the git user's home on the server.

    if let Some(shared_lib) = config.project.shared_lib.as_deref() {
        let shared_lib_path = std::path::Path::new(shared_lib);
        if !shared_lib_path.exists() {
            return Err(anyhow!(fail_message(&format!(
                "Shared lib path '{}' does not exist.",
                shared_lib
            ))));
        }

        let mut lib_spinner = Spinner::new(
            Spinners::SimpleDotsScrolling,
            succeed_message(&format!("Uploading shared lib {}…", shared_lib)),
        );

        let mut known_hosts_file = NamedTempFile::new()
            .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
        writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
            .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

        let ssh_command = build_ssh_command(&identity_file_str, &known_hosts_file);

        let output = Command::new("rsync")
            .args([
                "-r",
                "-e",
                &ssh_command,
                &format!("./{}", shared_lib),
                &format!("git@{}:~/", rsync_host),
            ])
            .output()
            .map_err(|e| anyhow!(fail_message(&format!("Failed to launch rsync: {}", e))))?;

        drop(known_hosts_file);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            lib_spinner.stop_and_persist(&fail_symbol(), fail_message("Shared lib upload failed."));
            return Err(anyhow!(fail_message(&format!(
                "rsync of '{}' failed: {}",
                shared_lib,
                stderr.trim()
            ))));
        }

        lib_spinner.stop_and_persist(&succeed_symbol(), succeed_message("Shared lib uploaded."));
    }

    // ── Step 2: compile native extensions on the server ──────────────────────
    //
    // If `compile_cmd` is set, run it on the server via SSH. This is typically
    // used to compile Rust-based gems that are part of the shared lib.

    if let Some(compile_cmd) = config.project.compile_cmd.as_deref() {
        let mut compile_spinner = Spinner::new(
            Spinners::SimpleDotsScrolling,
            succeed_message("Compiling native extensions on server…"),
        );

        let mut known_hosts_file = NamedTempFile::new()
            .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
        writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
            .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

        let compile_script = format!(
            r#"set -e
source ~/.profile 2>/dev/null || true
source ~/.bashrc 2>/dev/null || true
{compile_cmd}
"#,
            compile_cmd = compile_cmd,
        );

        let mut child = Command::new("ssh")
            .args(build_ssh_args(
                &identity_file_str,
                &known_hosts_file,
                &rsync_host,
            ))
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!(fail_message(&format!("Failed to spawn SSH: {}", e))))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(compile_script.as_bytes())
                .map_err(|e| anyhow!("Failed to write compile script to SSH stdin: {}", e))?;
        }

        let ssh_status = child
            .wait()
            .map_err(|e| anyhow!("Failed to wait for SSH process: {}", e))?;

        drop(known_hosts_file);

        if !ssh_status.success() {
            compile_spinner.stop_and_persist(
                &fail_symbol(),
                fail_message("Compilation failed on server. See output above."),
            );
            return Err(anyhow!(fail_message(&format!(
                "Remote compile command exited with status {}",
                ssh_status
            ))));
        }

        compile_spinner.stop_and_persist(
            &succeed_symbol(),
            succeed_message("Native extensions compiled."),
        );
    }

    // ── Step 3: record deployment as Started ─────────────────────────────────

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

    // ── Step 4: git init + force-push the sub-project directory ──────────────
    //
    // In a monorepo, the sub-project is not the git root. The server expects
    // a push of just the sub-project as if it were a standalone repo. We
    // create a temporary git repo inside the sub-project, commit everything,
    // and force-push to the server's bare repo. This mirrors the GA workflow:
    //
    //   cd backend/musik88-web
    //   git init && git add . && git commit -m "Deploy" && git push --force
    //
    // The temporary .git directory is cleaned up after the push.

    let git_host = runner.git_host();
    let remote_url = format!("{}:{}.git", git_host, repository);

    let mut push_spinner = Spinner::new(
        Spinners::Hamburger,
        succeed_message(&format!("Deploying {} → {}…", source, remote_url)),
    );

    // Use system git rather than libgit2 — we need SSH credential support
    // via the smbCloud identity file, and force-push to a temporary remote.
    // The GIT_SSH_COMMAND env var injects our hardened SSH options.

    let mut known_hosts_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
        .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    let git_ssh_command = build_ssh_command(&identity_file_str, &known_hosts_file);

    // Helper to run a git command in the source directory with our SSH config.
    let run_git = |args: &[&str]| -> Result<()> {
        let output = Command::new("git")
            .args(args)
            .current_dir(source)
            .env("GIT_SSH_COMMAND", &git_ssh_command)
            .output()
            .map_err(|e| anyhow!("Failed to run git {}: {}", args.join(" "), e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git {} failed: {}", args.join(" "), stderr.trim()));
        }
        Ok(())
    };

    // Check if the source directory is already a git repo (it shouldn't be —
    // it's a subdirectory of the monorepo). If a .git exists, it's leftover
    // from a previous deploy; remove it to start clean.
    let sub_git_dir = source_dir.join(".git");
    if sub_git_dir.exists() {
        std::fs::remove_dir_all(&sub_git_dir)
            .map_err(|e| anyhow!("Failed to remove leftover .git in '{}': {}", source, e))?;
    }

    let push_result = (|| -> Result<()> {
        run_git(&["init", "-b", "main"])?;
        run_git(&["add", "."])?;
        run_git(&[
            "-c",
            "user.email=deploy@smbcloud.xyz",
            "-c",
            "user.name=smb-deploy",
            "commit",
            "-m",
            "Deploy to production",
        ])?;
        run_git(&["remote", "add", "smbcloud", &remote_url])?;
        run_git(&["push", "--set-upstream", "smbcloud", "main", "--force"])?;
        Ok(())
    })();

    // Always clean up the temporary .git directory, even if the push failed.
    if sub_git_dir.exists() {
        let _ = std::fs::remove_dir_all(&sub_git_dir);
    }

    drop(known_hosts_file);

    match push_result {
        Ok(()) => {
            push_spinner.stop_and_persist(&succeed_symbol(), succeed_message("Push complete."));
        }
        Err(e) => {
            push_spinner.stop_and_persist(&fail_symbol(), fail_message("Push failed."));
            mark_failed(
                &deploy_ref,
                &created_deployment,
                &config,
                env,
                &access_token,
            )
            .await;
            return Err(anyhow!(fail_message(&format!("{}", e))));
        }
    }

    // ── Step 5: mark deployment as Done ──────────────────────────────────────

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

/// Build the SSH command string used for rsync's `-e` flag.
fn build_ssh_command(identity_file: &str, known_hosts_file: &NamedTempFile) -> String {
    format!(
        "ssh -i {identity} \
         -o StrictHostKeyChecking=yes \
         -o UserKnownHostsFile={known_hosts} \
         -o IdentitiesOnly=yes \
         -o PasswordAuthentication=no \
         -o BatchMode=yes",
        identity = identity_file,
        known_hosts = known_hosts_file.path().display(),
    )
}

/// Build SSH argument list for direct `Command::new("ssh")` invocations.
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

/// Mark the in-flight deployment record as Failed.
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
