//! Static deploy over HTTPS: build (or reuse) the output, hash it, upload the
//! blobs the server does not have, and wait for it to go live.
//!
//! Nothing here reads an SSH key, resolves a host or runs a command on the box.
//! The only credential is a token: `SMB_TOKEN` (an app-scoped deploy token in
//! CI) or the login token. The server chooses where the files land.

use {
    crate::{
        cli::CommandResult,
        deploy::process_deploy::DeployOptions,
        ui::{fail_message, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    smbcloud_deploy::{
        build_manifest, BuildStrategy, HttpsArtifactTransport, SvelteKitBuild, Transport,
        ViteSpaBuild,
    },
    smbcloud_model::{deploy_config::DeployConfig, project::DeploymentMethod},
    smbcloud_network::environment::Environment,
    smbcloud_utils::config::Config,
    spinners::{Spinner, Spinners},
    std::path::PathBuf,
};

/// The environment variable that carries a CI deploy token.
pub(crate) const TOKEN_ENV: &str = "SMB_TOKEN";

/// A non-empty `SMB_TOKEN`, if set.
pub(crate) fn env_token() -> Option<String> {
    std::env::var(TOKEN_ENV)
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Whether the server publishes this app as an artifact.
pub(crate) fn is_artifact(remote: &DeployConfig) -> bool {
    remote.publish_target.as_deref() == Some("artifact")
}

/// Plain static, `vite-spa` and `sveltekit` publish over HTTPS.
pub(crate) fn is_static_kind(config: &Config) -> bool {
    match config.project.kind.as_deref() {
        Some("vite-spa") | Some("sveltekit") => true,
        None => config.project.deployment_method == DeploymentMethod::Rsync,
        Some(_) => false,
    }
}

fn project_path(config: &Config) -> String {
    config.project.source.as_deref().unwrap_or(".").to_string()
}

fn package_manager(config: &Config) -> String {
    config
        .project
        .package_manager
        .as_deref()
        .unwrap_or("pnpm")
        .to_string()
}

/// The build strategy for the app's kind, or `None` for a plain static site.
fn strategy(config: &Config) -> Option<Box<dyn BuildStrategy>> {
    match config.project.kind.as_deref() {
        Some("vite-spa") => Some(Box::new(ViteSpaBuild {
            project_path: project_path(config),
            output_dir: config
                .project
                .output
                .as_deref()
                .unwrap_or("dist")
                .to_string(),
            package_manager: package_manager(config),
        })),
        Some("sveltekit") => Some(Box::new(SvelteKitBuild {
            project_path: project_path(config),
            output_dir: config
                .project
                .output
                .as_deref()
                .unwrap_or("build")
                .to_string(),
            package_manager: package_manager(config),
        })),
        _ => None,
    }
}

/// Where the output lives when the build is skipped.
fn prebuilt_dir(config: &Config) -> PathBuf {
    let base = PathBuf::from(project_path(config));
    match config.project.kind.as_deref() {
        Some("vite-spa") => base.join(config.project.output.as_deref().unwrap_or("dist")),
        Some("sveltekit") => base.join(config.project.output.as_deref().unwrap_or("build")),
        _ => PathBuf::from("."),
    }
}

fn head_sha() -> Option<String> {
    let repo = git2::Repository::open(".").ok()?;
    let id = repo.head().ok()?.target()?;
    Some(id.to_string())
}

fn api_base(env: Environment) -> String {
    format!("{}://{}", env.api_protocol(), env.api_host())
}

pub(crate) async fn process_deploy_artifact(
    env: Environment,
    config: Config,
    remote: DeployConfig,
    token: String,
    options: DeployOptions,
) -> Result<CommandResult> {
    if !is_static_kind(&config) {
        return Err(anyhow!(fail_message(&format!(
            "'{}' publishes over HTTPS, which supports static sites only \
             (plain static, vite-spa, sveltekit). Kind: {}.",
            config.project.name,
            config.project.kind.as_deref().unwrap_or("none")
        ))));
    }

    let reporter = crate::ui::reporter::SpinnerReporter::new();
    let source_dir = if options.prebuilt {
        prebuilt_dir(&config)
    } else if let Some(strategy) = strategy(&config) {
        strategy
            .build(&reporter)
            .map_err(|e| anyhow!(fail_message(&e.to_string())))?
            .source_dir
    } else {
        PathBuf::from(".")
    };

    if options.dry_run {
        return dry_run(&config, &remote, &source_dir);
    }

    let transport =
        HttpsArtifactTransport::new(api_base(env), remote.id.clone(), token, head_sha());
    let ship_dir = source_dir.clone();
    let shipped = tokio::task::spawn_blocking(move || {
        let reporter = crate::ui::reporter::SpinnerReporter::new();
        let result = transport.ship(&ship_dir, &reporter);
        (transport, result)
    })
    .await
    .map_err(|e| anyhow!(fail_message(&format!("deploy task failed: {e}"))))?;

    let (transport, result) = shipped;
    result.map_err(|e| anyhow!(fail_message(&e.to_string())))?;

    let url = transport
        .deployment()
        .and_then(|d| d.url)
        .or(remote.site_url.clone());
    let message = match url {
        Some(url) => format!("Deployed {}", url),
        None => "Deployment complete.".to_string(),
    };
    let spinner = Spinner::new(Spinners::Hamburger, succeed_message(&message));
    Ok(CommandResult {
        spinner,
        symbol: succeed_symbol(),
        msg: succeed_message(&message),
    })
}

/// Prints what would be uploaded and stops before anything leaves the machine.
fn dry_run(
    config: &Config,
    remote: &DeployConfig,
    source_dir: &std::path::Path,
) -> Result<CommandResult> {
    let manifest = build_manifest(source_dir).map_err(|e| anyhow!(fail_message(&e.to_string())))?;
    println!("App:     {} ({})", remote.name, remote.id);
    println!("Project: {}", config.project.name);
    println!("Source:  {}", source_dir.display());
    println!(
        "Target:  {}",
        remote
            .site_url
            .as_deref()
            .unwrap_or("allocated by smbCloud when the deployment is finalized")
    );
    for file in &manifest.files {
        println!("  {:>10}  {}", file.entry.size, file.entry.path);
    }
    let message = format!(
        "Dry run: {} files, {} bytes. Nothing was uploaded.",
        manifest.files.len(),
        manifest.total_bytes()
    );
    let spinner = Spinner::new(Spinners::Hamburger, succeed_message(&message));
    Ok(CommandResult {
        spinner,
        symbol: succeed_symbol(),
        msg: succeed_message(&message),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(kind: Option<&str>, method: DeploymentMethod) -> Config {
        let toml = format!(
            "name = \"x\"\n[project]\nid = 1\nname = \"x\"\nrunner = 1\ndeployment_method = {}\n{}",
            method as u8,
            kind.map(|k| format!("kind = \"{k}\"\n"))
                .unwrap_or_default()
        );
        toml::from_str(&toml).expect("config parses")
    }

    #[test]
    fn only_static_kinds_use_the_artifact_path() {
        assert!(is_static_kind(&config(
            Some("vite-spa"),
            DeploymentMethod::Rsync
        )));
        assert!(is_static_kind(&config(
            Some("sveltekit"),
            DeploymentMethod::Rsync
        )));
        assert!(is_static_kind(&config(None, DeploymentMethod::Rsync)));
        assert!(!is_static_kind(&config(None, DeploymentMethod::Git)));
        for kind in ["nextjs-ssr", "astro-ssr", "rails", "rust", "swift"] {
            assert!(!is_static_kind(&config(
                Some(kind),
                DeploymentMethod::Rsync
            )));
        }
    }

    #[test]
    fn prebuilt_ships_the_output_dir() {
        assert_eq!(
            prebuilt_dir(&config(Some("vite-spa"), DeploymentMethod::Rsync)),
            PathBuf::from("./dist")
        );
        assert_eq!(
            prebuilt_dir(&config(Some("sveltekit"), DeploymentMethod::Rsync)),
            PathBuf::from("./build")
        );
        assert_eq!(
            prebuilt_dir(&config(None, DeploymentMethod::Rsync)),
            PathBuf::from(".")
        );
    }

    #[tokio::test]
    async fn an_artifact_dry_run_touches_no_ssh_and_uploads_nothing() {
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", home.path());
        let remote: DeployConfig = serde_json::from_str(
            r#"{"id":"app1","name":"site","runner":1,"deployment_method":1,"publish_target":"artifact"}"#,
        )
        .unwrap();

        let result = process_deploy_artifact(
            Environment::Dev,
            config(None, DeploymentMethod::Rsync),
            remote,
            "smb_deploy_x".to_string(),
            DeployOptions {
                dry_run: true,
                prebuilt: true,
            },
        )
        .await;

        assert!(result.is_ok());
        assert!(!home.path().join(".ssh").exists());
    }
}
