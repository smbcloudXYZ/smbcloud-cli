pub mod config;
mod git;
pub mod process_deploy;
pub mod process_deploy_nextjs_ssr;
pub mod process_deploy_rails;
pub mod process_deploy_rust;
pub mod process_deploy_swift;
pub mod process_deploy_vite_spa;
pub mod process_migrate;
mod remote_messages;
pub(crate) mod setup_create_new_project;
pub(crate) mod setup_project;
pub(crate) mod setup_select_project;

use {
    anyhow::{anyhow, Result},
    smbcloud_deploy::RsyncTransport,
    smbcloud_model::runner::Runner,
    smbcloud_utils::config::Config,
};

// The pinned host keys live in the engine crate. Re-export them here so the
// strategy modules that build their own pinned-SSH commands can keep using
// `deploy::known_hosts`.
pub(crate) use smbcloud_deploy::known_hosts;

/// Build the rsync transport for the current project.
///
/// This resolves the front-end-specific bits the engine deliberately doesn't
/// know about: the server host (from the runner), the remote path (from config,
/// defaulting to `apps/web/<name>`), and the user's local SSH identity file.
pub(crate) fn rsync_transport(
    config: &Config,
    runner: &Runner,
    user_id: i32,
) -> Result<RsyncTransport> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let identity_file = home.join(".ssh").join(format!("id_{user_id}@smbcloud"));

    let remote_path = match &config.project.path {
        Some(path) => path.clone(),
        None => format!("apps/web/{}", config.project.name),
    };

    Ok(RsyncTransport::new(
        runner.rsync_host(),
        remote_path,
        identity_file,
    ))
}
