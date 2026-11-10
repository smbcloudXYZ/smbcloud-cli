//! `smb gresiq`: a project's GresIQ app (the document store behind Flags and
//! Onde telemetry). The same calls back the `gresiq_app_*` MCP tools.
//!
//! Secrets are never printed. The API secret from `create` and the keys from
//! `rotate-server-key` / `rotate-ingest-key` are shown by smbCloud exactly
//! once, so they go to a file you name, created with mode 0600. Without a file
//! the command refuses to rotate (the old key would already be dead) and
//! `create` keeps the secret out of the terminal.

use crate::{
    cli::CommandResult,
    client,
    flags::{authorized, done, project_id, send},
    session_config,
    token::get_smb_token::get_smb_token,
};
use anyhow::{anyhow, Result};
use clap::Subcommand;
use console::style;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use smbcloud_model::gresiq::{GresiqApp, GresiqAppCredentials, RotatedKey};
use smbcloud_network::environment::Environment;
use smbcloud_networking::smb_base_url_builder;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "List the GresIQ apps in the selected tenant.")]
    List {
        #[clap(short, long, help = "Only this project's app.")]
        project: Option<String>,
    },
    #[clap(about = "Show one GresIQ app.")]
    Show { id: String },
    #[clap(
        about = "Create the project's GresIQ app. A project has at most one; the first smbCloud Flag creates it too."
    )]
    Create {
        #[clap(short, long, help = "Project id. Defaults to the selected project.")]
        project: Option<String>,
        #[clap(long, help = "Display name. Defaults to `<project>-GresIQ`.")]
        name: Option<String>,
        #[clap(
            long,
            help = "Write GRESIQ_API_KEY and GRESIQ_API_SECRET to this file (mode 0600). The secret is shown only once."
        )]
        credentials_file: Option<PathBuf>,
    },
    #[clap(
        about = "Issue a new gqs_ server key for a trusted backend. The previous key stops working."
    )]
    RotateServerKey {
        id: String,
        #[clap(long, help = "File to write the key to (mode 0600). Required.")]
        output: PathBuf,
    },
    #[clap(about = "Issue a new write-only gqi_ ingest key. The previous key stops working.")]
    RotateIngestKey {
        id: String,
        #[clap(long, help = "File to write the key to (mode 0600). Required.")]
        output: PathBuf,
    },
}

pub async fn process_gresiq(env: Environment, commands: Commands) -> Result<CommandResult> {
    let token = get_smb_token(env)?;
    match commands {
        Commands::List { project } => {
            let apps = list_apps(env, &token, project.as_deref()).await?;
            print_apps(&apps);
            Ok(done(if apps.is_empty() {
                "No GresIQ apps yet."
            } else {
                "Done."
            }))
        }
        Commands::Show { id } => {
            print_app(&get_app(env, &token, &id).await?);
            Ok(done("Done."))
        }
        Commands::Create {
            project,
            name,
            credentials_file,
        } => {
            if let Some(path) = &credentials_file {
                refuse_existing(path)?;
            }
            let project = project_id(env, project)?;
            let (app, credentials) = create_app(env, &token, &project, name).await?;
            print_app(&app);
            match (&credentials_file, credentials.api_secret.as_deref()) {
                (Some(path), Some(secret)) => {
                    let key = credentials.api_key.clone().unwrap_or_default();
                    write_private(
                        path,
                        &format!("GRESIQ_API_KEY={key}\nGRESIQ_API_SECRET={secret}\n"),
                    )?;
                    Ok(done(&format!(
                        "GresIQ app created. Credentials written to {}.",
                        path.display()
                    )))
                }
                _ => Ok(done(
                    "GresIQ app created. The API secret was not saved; pass --credentials-file next time, or use a server key (smb gresiq rotate-server-key).",
                )),
            }
        }
        Commands::RotateServerKey { id, output } => {
            refuse_existing(&output)?;
            let key = rotate_key(env, &token, &id, KeyKind::Server).await?;
            write_private(&output, &format!("{}\n", key.key))?;
            Ok(done(&format!(
                "New server key …{} written to {}. The previous key no longer works.",
                key.last4,
                output.display()
            )))
        }
        Commands::RotateIngestKey { id, output } => {
            refuse_existing(&output)?;
            let key = rotate_key(env, &token, &id, KeyKind::Ingest).await?;
            write_private(&output, &format!("{}\n", key.key))?;
            Ok(done(&format!(
                "New ingest key …{} written to {}. The previous key no longer works.",
                key.last4,
                output.display()
            )))
        }
    }
}

fn apps_url(env: Environment, suffix: &[&str], query: &[(&str, &str)]) -> Result<String> {
    let mut builder = smb_base_url_builder(env, client());
    builder.add_route("v1/gresiq_apps");
    for part in suffix {
        builder.add_route(part);
    }
    for (key, value) in query {
        builder.add_param(key, value);
    }
    // Same as Flags: the selected tenant travels with every request.
    if let Some(tenant_id) = session_config::current_tenant_id(env)? {
        builder.add_param("tenant_id", &tenant_id);
    }
    Ok(builder.build())
}

pub(crate) async fn list_apps(
    env: Environment,
    token: &str,
    project: Option<&str>,
) -> Result<Vec<GresiqApp>> {
    let query: Vec<(&str, &str)> = project.map(|p| ("project_id", p)).into_iter().collect();
    let url = apps_url(env, &[], &query)?;
    send(authorized(Client::new().get(url), token)).await
}

pub(crate) async fn get_app(env: Environment, token: &str, id: &str) -> Result<GresiqApp> {
    let url = apps_url(env, &[id], &[])?;
    send(authorized(Client::new().get(url), token)).await
}

#[derive(Serialize)]
struct CreateBody<'a> {
    gresiq_app: CreateApp<'a>,
}

#[derive(Serialize)]
struct CreateApp<'a> {
    project_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

/// Create the app. The credentials come back separately so a caller can't
/// print them by printing the app.
pub(crate) async fn create_app(
    env: Environment,
    token: &str,
    project: &str,
    name: Option<String>,
) -> Result<(GresiqApp, GresiqAppCredentials)> {
    let url = apps_url(env, &[], &[])?;
    let body = CreateBody {
        gresiq_app: CreateApp {
            project_id: project,
            name: name.filter(|n| !n.trim().is_empty()),
        },
    };
    let value: serde_json::Value =
        send(authorized(Client::new().post(url), token).json(&body)).await?;
    let credentials: GresiqAppCredentials = serde_json::from_value(value.clone())?;
    let app: GresiqApp = serde_json::from_value(value)?;
    Ok((app, credentials))
}

#[derive(Clone, Copy)]
enum KeyKind {
    Server,
    Ingest,
}

#[derive(Deserialize)]
struct RotatedServer {
    server_key: String,
    server_key_last4: String,
}

#[derive(Deserialize)]
struct RotatedIngest {
    ingest_key: String,
    ingest_key_last4: String,
}

async fn rotate_key(env: Environment, token: &str, id: &str, kind: KeyKind) -> Result<RotatedKey> {
    let action = match kind {
        KeyKind::Server => "rotate_server_key",
        KeyKind::Ingest => "rotate_ingest_key",
    };
    let request = authorized(
        Client::new().post(apps_url(env, &[id, action], &[])?),
        token,
    );
    Ok(match kind {
        KeyKind::Server => {
            let r: RotatedServer = send(request).await?;
            RotatedKey {
                key: r.server_key,
                last4: r.server_key_last4,
            }
        }
        KeyKind::Ingest => {
            let r: RotatedIngest = send(request).await?;
            RotatedKey {
                key: r.ingest_key,
                last4: r.ingest_key_last4,
            }
        }
    })
}

/// Rotating kills the old key, so check the output path before asking.
fn refuse_existing(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(anyhow!(
            "{} already exists. Pick a new file, so a key is never overwritten or mixed with other content.",
            path.display()
        ));
    }
    Ok(())
}

/// Create `path` with mode 0600 and write `contents`. Fails if it exists.
pub(crate) fn write_private(path: &Path, contents: &str) -> Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|e| anyhow!("Couldn't create {}: {e}", path.display()))?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

fn print_apps(apps: &[GresiqApp]) {
    println!("\n{}", style("GresIQ apps").bold().underlined());
    if apps.is_empty() {
        println!("No GresIQ apps yet.");
        return;
    }
    for app in apps {
        println!(
            "{} {} (project {}, {})",
            style(app.id).bold(),
            app.name,
            app.project_id,
            app.status
        );
    }
}

fn print_app(app: &GresiqApp) {
    let field =
        |label: &str, value: String| println!("{} {value}", style(format!("{label}:")).cyan());
    println!("\n{}", style("GresIQ app").bold().underlined());
    field("ID", app.id.to_string());
    field("Name", app.name.clone());
    field("Project", app.project_id.to_string());
    field("Tenant", app.tenant_id.to_string());
    field("Status", app.status.clone());
    field("Table prefix", app.table_prefix.clone());
    let last4 = |value: &Option<String>| {
        value
            .as_deref()
            .map(|l| format!("…{l}"))
            .unwrap_or_else(|| "none".into())
    };
    field("Server key", last4(&app.server_key_last4));
    field("Ingest key", last4(&app.ingest_key_last4));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let dir = tempfile::tempdir().unwrap().keep();
        dir.join(name)
    }

    #[test]
    fn keys_are_written_owner_only_and_never_over_an_existing_file() {
        let path = temp_path("gqs");
        write_private(&path, "gqs_abc\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "gqs_abc\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert!(write_private(&path, "other").is_err());
        assert!(refuse_existing(&path).is_err());
    }

    #[test]
    fn create_body_carries_the_project_and_an_optional_name() {
        let body = CreateBody {
            gresiq_app: CreateApp {
                project_id: "38",
                name: None,
            },
        };
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            serde_json::json!({ "gresiq_app": { "project_id": "38" } })
        );
    }
}
