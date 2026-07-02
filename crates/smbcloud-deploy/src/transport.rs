//! Shipping a source tree to its deploy target.
//!
//! A [`Transport`] takes files that are ready to ship and puts them on the
//! server. Today the only one is [`RsyncTransport`] (rsync over SSH with the
//! server host key pinned). A git-smart-HTTP transport can implement the same
//! trait later without changing any callers.

use crate::{error::DeployError, known_hosts, report::Reporter};
use anyhow::anyhow;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::NamedTempFile;

/// Ships a source tree to its deploy target.
pub trait Transport {
    /// Transfer `source` to the target, reporting progress via `reporter`.
    fn ship(&self, source: &Path, reporter: &dyn Reporter) -> Result<(), DeployError>;
}

/// rsync over SSH with the server's host key pinned (see [`known_hosts`]).
///
/// Auth and topology are resolved by the caller and passed in: the front-end
/// owns where the identity file lives and how the remote path is derived, so the
/// engine makes no assumptions about `~/.ssh` layout or project naming.
pub struct RsyncTransport {
    host: String,
    remote_path: String,
    identity_file: PathBuf,
}

impl RsyncTransport {
    pub fn new(host: String, remote_path: String, identity_file: PathBuf) -> Self {
        Self {
            host,
            remote_path,
            identity_file,
        }
    }
}

impl Transport for RsyncTransport {
    fn ship(&self, source: &Path, reporter: &dyn Reporter) -> Result<(), DeployError> {
        // Pin the server's host key to a temp known_hosts file for this transfer,
        // protecting every user against DNS/BGP hijacking even on untrusted
        // networks. The file is deleted when it drops at the end of this call.
        let mut known_hosts_file = NamedTempFile::new()
            .map_err(|e| anyhow!("Failed to create temp known_hosts file: {e}"))?;
        writeln!(known_hosts_file, "{}", known_hosts::for_host(&self.host))
            .map_err(|e| anyhow!("Failed to write known_hosts: {e}"))?;

        // Fully hardened SSH: only the pinned host key is trusted, only the given
        // identity is used, no password fallback, never prompt.
        let ssh_command = format!(
            "ssh -i {identity} \
             -o StrictHostKeyChecking=yes \
             -o UserKnownHostsFile={known_hosts} \
             -o IdentitiesOnly=yes \
             -o PasswordAuthentication=no \
             -o BatchMode=yes",
            identity = self.identity_file.to_string_lossy(),
            known_hosts = known_hosts_file.path().display(),
        );

        // `rsync -a src/` syncs contents, not the directory itself, so both
        // sides need a trailing slash.
        let source_str = source.to_string_lossy();
        let source_with_slash = if source_str.ends_with('/') {
            source_str.into_owned()
        } else {
            format!("{source_str}/")
        };
        let remote_with_slash = if self.remote_path.ends_with('/') {
            self.remote_path.clone()
        } else {
            format!("{}/", self.remote_path)
        };
        let destination = format!("git@{}:{}", self.host, remote_with_slash);

        reporter.step_start(&format!("Syncing {source_with_slash} -> {destination}"));

        // known_hosts_file must stay alive until rsync exits so SSH can read it.
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
        drop(known_hosts_file);

        match output {
            Ok(result) if result.status.success() => {
                for line in String::from_utf8_lossy(&result.stderr).lines() {
                    reporter.remote_line(line);
                }
                reporter.step_done("Deployment complete via rsync.");
                Ok(())
            }
            Ok(result) => {
                let code = result.status.code().unwrap_or(-1);
                reporter.step_fail(&format!("rsync exited with status {code}"));
                for line in String::from_utf8_lossy(&result.stderr).lines() {
                    reporter.remote_line(line);
                }
                Err(DeployError::Other(anyhow!(
                    "rsync exited with status {code}"
                )))
            }
            Err(e) => {
                reporter.step_fail(&format!("Failed to launch rsync: {e}. Is rsync installed?"));
                Err(DeployError::Other(anyhow!("Failed to launch rsync: {e}")))
            }
        }
    }
}
