use {
    crate::{
        cli::CommandResult,
        deploy::known_hosts,
        ui::{fail_message, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    smbcloud_model::runner::Runner,
    smbcloud_utils::config::Config,
    spinners::{Spinner, Spinners},
    std::{io::Write, process::Command},
    tempfile::NamedTempFile,
};

/// Runs an rsync transfer replicating:
///   rsync -a --exclude='.git' --exclude='.smb' \
///         -e "ssh -i ~/.ssh/id_<user_id>@smbcloud
///              -o StrictHostKeyChecking=yes
///              -o UserKnownHostsFile=<temp_pinned_known_hosts>
///              -o IdentitiesOnly=yes
///              -o PasswordAuthentication=no
///              -o BatchMode=yes" \
///         <source>/ git@<api-host>.smbcloud.xyz:<remote_path>/
///
/// # Why std::process::Command instead of embedding::run_client
///
/// `embedding::run_client` drives the oc-rsync client, which sends extension
/// message codes (e.g. code 90, wire tag 97) that standard rsync 3.2.7 on the
/// server does not recognise, causing it to abort with:
///
///   unexpected tag 97 [Receiver]
///   rsync error: error in rsync protocol data stream (code 12)
///
/// oc-rsync is designed for oc-rsync ↔ oc-rsync transfers. Using it against a
/// stock rsync server is outside its design envelope without patching the
/// protocol layer. System rsync speaks exactly what the server expects.
///
/// # Host key pinning
///
/// The server's ed25519 public key is embedded in `known_hosts.rs` and written
/// to a temp file at deploy time. SSH is told to use that file exclusively
/// (`UserKnownHostsFile`) and to refuse any key that doesn't match
/// (`StrictHostKeyChecking=yes`). This protects every user — not just the
/// developer — against DNS/BGP hijacking, even on untrusted networks. The temp
/// file is deleted automatically when this function returns.
pub fn rsync_deploy(
    config: &Config,
    runner: &Runner,
    user_id: i32,
    source: &str,
) -> Result<CommandResult> {
    // Identity file: ~/.ssh/id_<user_id>@smbcloud
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let identity_file = home.join(".ssh").join(format!("id_{}@smbcloud", user_id));
    let identity_file_str = identity_file.to_string_lossy().into_owned();

    // Write the pinned host key to a temp known_hosts file.
    // NamedTempFile is deleted automatically when it drops at end of this fn.
    let rsync_host = runner.rsync_host();
    let mut known_hosts_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
        .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    // Fully hardened SSH command:
    //   StrictHostKeyChecking=yes  — refuse any host not in our pinned known_hosts
    //   UserKnownHostsFile=<temp>  — only trust our pinned key, not ~/.ssh/known_hosts
    //   IdentitiesOnly=yes         — use only the specified key, ignore ssh-agent
    //   PasswordAuthentication=no  — no fallback to password auth
    //   BatchMode=yes              — never prompt; fail fast
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

    // Remote destination path: use project.path if set, otherwise derive from name.
    let remote_path = match &config.project.path {
        Some(path) => path.clone(),
        None => format!("apps/web/{}", config.project.name),
    };

    // rsync -a src/ syncs contents, not the directory itself —
    // both source and destination must have a trailing slash.
    let source_with_slash = if source.ends_with('/') {
        source.to_owned()
    } else {
        format!("{}/", source)
    };

    let remote_with_slash = if remote_path.ends_with('/') {
        remote_path
    } else {
        format!("{}/", remote_path)
    };

    let destination = format!("git@{}:{}", rsync_host, remote_with_slash);

    let spinner = Spinner::new(
        Spinners::Hamburger,
        succeed_message(&format!("Syncing {} -> {}", source_with_slash, destination)),
    );

    // known_hosts_file must remain alive until rsync exits so SSH can read it.
    let output = Command::new("rsync")
        .args([
            "-a",
            "--exclude=.git",
            "--exclude=.smb",
            "-e",
            &ssh_command,
            &source_with_slash,
            &destination,
        ])
        .output();

    // Drop the temp known_hosts file now that the process has finished.
    drop(known_hosts_file);

    match output {
        Ok(result) if result.status.success() => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            if !stderr.is_empty() {
                for line in stderr.lines() {
                    println!("{}", line);
                }
            }
            Ok(CommandResult {
                spinner,
                symbol: succeed_symbol(),
                msg: succeed_message("Deployment complete via rsync."),
            })
        }
        Ok(result) => {
            drop(spinner);
            let stderr = String::from_utf8_lossy(&result.stderr);
            if !stderr.is_empty() {
                eprintln!("{}", stderr);
            }
            Err(anyhow!(fail_message(&format!(
                "rsync exited with status {}",
                result.status.code().unwrap_or(-1)
            ))))
        }
        Err(e) => {
            drop(spinner);
            Err(anyhow!(fail_message(&format!(
                "Failed to launch rsync: {}. Is rsync installed?",
                e
            ))))
        }
    }
}
