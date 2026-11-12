//! `smb flags`: smbCloud Flags, the boolean flags a project keeps in its GresIQ
//! app. The same calls back the `flags_*` MCP tools.

use crate::{
    cli::CommandResult,
    client, session_config,
    token::get_smb_token::get_smb_token,
    ui::{succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use clap::Subcommand;
use console::style;
use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use smbcloud_model::flags::{Flag, FlagEnvelope, FlagInput, FlagList, FlagTargetsInput};
use smbcloud_network::environment::Environment;
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_base_url_builder};
use spinners::Spinner;

#[derive(clap::Args, Clone, Debug, Default)]
pub struct FlagFields {
    #[clap(long, help = "Display name.")]
    pub name: Option<String>,
    #[clap(long, help = "What the flag controls.")]
    pub description: Option<String>,
    #[clap(long, conflicts_with = "off", help = "Switch the flag on.")]
    pub on: bool,
    #[clap(long, conflicts_with = "on", help = "Switch the flag off.")]
    pub off: bool,
    #[clap(
        long,
        conflicts_with = "not_overridable",
        help = "Let a visitor force a variant with ?treatment= or ?control=."
    )]
    pub overridable: bool,
    #[clap(
        long,
        conflicts_with = "overridable",
        help = "Stop a visitor forcing a variant."
    )]
    pub not_overridable: bool,
    #[clap(
        long,
        value_delimiter = ',',
        help = "Emails it is on for, comma separated. Replaces the list."
    )]
    pub emails: Option<Vec<String>>,
    #[clap(
        long,
        value_delimiter = ',',
        help = "Email domains it is on for, comma separated. Replaces the list."
    )]
    pub domains: Option<Vec<String>>,
    #[clap(
        long,
        value_parser = clap::value_parser!(u8).range(0..=100),
        help = "A stable share of all users, 0 to 100."
    )]
    pub percent: Option<u8>,
}

impl FlagFields {
    pub fn into_input(self, key: Option<String>) -> FlagInput {
        FlagInput {
            key,
            name: self.name,
            description: self.description,
            on: match (self.on, self.off) {
                (true, _) => Some(true),
                (_, true) => Some(false),
                _ => None,
            },
            overridable: match (self.overridable, self.not_overridable) {
                (true, _) => Some(true),
                (_, true) => Some(false),
                _ => None,
            },
            targets: FlagTargetsInput {
                emails: self.emails,
                domains: self.domains,
                percent: self.percent,
            },
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "List a project's flags.")]
    List {
        #[clap(short, long, help = "Project id. Defaults to the selected project.")]
        project: Option<String>,
    },
    #[clap(about = "Show one flag.")]
    Get {
        key: String,
        #[clap(short, long)]
        project: Option<String>,
    },
    #[clap(about = "Create a flag. The project needs a GresIQ app.")]
    Create {
        #[clap(help = "Lowercase letters, digits and underscores, starting with a letter.")]
        key: String,
        #[clap(short, long)]
        project: Option<String>,
        #[clap(flatten)]
        fields: FlagFields,
    },
    #[clap(about = "Change a flag. Only the options you pass are changed.")]
    Update {
        key: String,
        #[clap(short, long)]
        project: Option<String>,
        #[clap(flatten)]
        fields: FlagFields,
    },
    #[clap(about = "Delete a flag.")]
    Delete {
        key: String,
        #[clap(short, long)]
        project: Option<String>,
        #[clap(long, help = "Confirm the deletion.")]
        yes: bool,
    },
}

pub async fn process_flags(env: Environment, commands: Commands) -> Result<CommandResult> {
    let token = get_smb_token(env)?;
    match commands {
        Commands::List { project } => {
            let flags = list_flags(env, &token, &project_id(env, project)?).await?;
            print_flags(&flags);
            Ok(done(if flags.is_empty() {
                "No flags yet."
            } else {
                "Done."
            }))
        }
        Commands::Get { key, project } => {
            let flag = get_flag(env, &token, &project_id(env, project)?, &key).await?;
            print_flag(&flag);
            Ok(done("Done."))
        }
        Commands::Create {
            key,
            project,
            fields,
        } => {
            let flag = create_flag(
                env,
                &token,
                &project_id(env, project)?,
                fields.into_input(Some(key)),
            )
            .await?;
            print_flag(&flag);
            Ok(done("Flag created."))
        }
        Commands::Update {
            key,
            project,
            fields,
        } => {
            let input = fields.into_input(None);
            if input == FlagInput::default() {
                return Err(anyhow!(
                    "Pass at least one of --name, --description, --on, --off, --overridable, --not-overridable, --emails, --domains or --percent."
                ));
            }
            let flag = update_flag(env, &token, &project_id(env, project)?, &key, input).await?;
            print_flag(&flag);
            Ok(done("Flag updated."))
        }
        Commands::Delete { key, project, yes } => {
            if !yes {
                return Err(anyhow!(
                    "This deletes the flag `{key}`. Pass --yes to confirm."
                ));
            }
            delete_flag(env, &token, &project_id(env, project)?, &key).await?;
            Ok(done("Flag deleted."))
        }
    }
}

pub(crate) fn project_id(env: Environment, project: Option<String>) -> Result<String> {
    let project = match project {
        Some(project) => Some(project),
        None => session_config::current_project_id(env)?,
    };
    project
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow!("Pass --project <id>, or select one with `smb project use`."))
}

fn flags_url(env: Environment, project_id: &str, key: Option<&str>) -> Result<String> {
    let mut builder = smb_base_url_builder(env, client());
    builder.add_route("v1/projects");
    builder.add_route(project_id);
    builder.add_route("flags");
    if let Some(key) = key {
        builder.add_route(key);
    }
    // The API resolves the project inside the caller's tenant, so a selected
    // tenant has to travel with the request.
    if let Some(tenant_id) = session_config::current_tenant_id(env)? {
        builder.add_param("tenant_id", &tenant_id);
    }
    Ok(builder.build())
}

pub(crate) fn authorized(builder: RequestBuilder, token: &str) -> RequestBuilder {
    builder
        .header("Authorization", token)
        .header("User-agent", SMB_USER_AGENT)
}

/// The API answers errors as `{ "errors": [..] }` or `{ "error": ".." }`. Say
/// what it said, since the message ("Set up GresIQ for this project first.")
/// is the useful part.
pub(crate) async fn send<T: DeserializeOwned>(builder: RequestBuilder) -> Result<T> {
    let response = builder
        .send()
        .await
        .map_err(|error| anyhow!("Couldn't reach smbCloud: {error}"))?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!(error_message(status.as_u16(), &body)));
    }
    serde_json::from_str(&body).map_err(|error| anyhow!("Unexpected answer from smbCloud: {error}"))
}

pub(crate) fn error_message(status: u16, body: &str) -> String {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let message = parsed.as_ref().and_then(|value| {
        value
            .get("errors")
            .and_then(Value::as_array)
            .map(|errors| {
                errors
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|text| !text.is_empty())
            .or_else(|| {
                ["error", "message"]
                    .iter()
                    .find_map(|field| value.get(field).and_then(Value::as_str))
                    .map(str::to_string)
            })
    });
    message.unwrap_or_else(|| format!("smbCloud answered {status}."))
}

pub(crate) async fn list_flags(env: Environment, token: &str, project: &str) -> Result<Vec<Flag>> {
    let builder = authorized(Client::new().get(flags_url(env, project, None)?), token);
    Ok(send::<FlagList>(builder).await?.flags)
}

pub(crate) async fn get_flag(
    env: Environment,
    token: &str,
    project: &str,
    key: &str,
) -> Result<Flag> {
    let builder = authorized(
        Client::new().get(flags_url(env, project, Some(key))?),
        token,
    );
    Ok(send::<FlagEnvelope>(builder).await?.flag)
}

#[derive(Serialize)]
struct FlagBody<'a> {
    flag: &'a FlagInput,
}

pub(crate) async fn create_flag(
    env: Environment,
    token: &str,
    project: &str,
    input: FlagInput,
) -> Result<Flag> {
    let builder = authorized(Client::new().post(flags_url(env, project, None)?), token)
        .json(&FlagBody { flag: &input });
    Ok(send::<FlagEnvelope>(builder).await?.flag)
}

pub(crate) async fn update_flag(
    env: Environment,
    token: &str,
    project: &str,
    key: &str,
    input: FlagInput,
) -> Result<Flag> {
    let builder = authorized(
        Client::new().patch(flags_url(env, project, Some(key))?),
        token,
    )
    .json(&FlagBody { flag: &input });
    Ok(send::<FlagEnvelope>(builder).await?.flag)
}

pub(crate) async fn delete_flag(
    env: Environment,
    token: &str,
    project: &str,
    key: &str,
) -> Result<()> {
    let response = authorized(
        Client::new().delete(flags_url(env, project, Some(key))?),
        token,
    )
    .send()
    .await
    .map_err(|error| anyhow!("Couldn't reach smbCloud: {error}"))?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    Err(anyhow!(error_message(
        status.as_u16(),
        &response.text().await?
    )))
}

fn print_flags(flags: &[Flag]) {
    println!("\n{}", style("Flags").bold().underlined());
    if flags.is_empty() {
        println!("No flags yet.");
        return;
    }
    for flag in flags {
        let state = if flag.on { "on " } else { "off" };
        println!("{state} {} ({})", style(&flag.key).bold(), flag.name);
    }
}

fn print_flag(flag: &Flag) {
    let field =
        |label: &str, value: String| println!("{} {value}", style(format!("{label}:")).cyan());
    println!("\n{}", style("Flag").bold().underlined());
    field("Key", flag.key.clone());
    field("Name", flag.name.clone());
    if !flag.description.is_empty() {
        field("Description", flag.description.clone());
    }
    field("On", flag.on.to_string());
    field("Overridable", flag.overridable.to_string());
    field("Emails", flag.targets.emails.join(", "));
    field("Domains", flag.targets.domains.join(", "));
    field("Percent", flag.targets.percent.to_string());
}

pub(crate) fn done(message: &str) -> CommandResult {
    CommandResult {
        spinner: Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            succeed_message("Done"),
        ),
        symbol: succeed_symbol(),
        msg: succeed_message(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_and_off_become_a_bool_and_unset_stays_unset() {
        let on = FlagFields {
            on: true,
            ..Default::default()
        };
        assert_eq!(on.into_input(None).on, Some(true));
        let off = FlagFields {
            off: true,
            ..Default::default()
        };
        assert_eq!(off.into_input(None).on, Some(false));
        assert_eq!(FlagFields::default().into_input(None), FlagInput::default());
    }

    #[test]
    fn api_errors_say_what_the_api_said() {
        assert_eq!(
            error_message(
                422,
                r#"{"errors":["Set up GresIQ for this project first."]}"#
            ),
            "Set up GresIQ for this project first."
        );
        assert_eq!(
            error_message(404, r#"{"error":"Flag not found"}"#),
            "Flag not found"
        );
        assert_eq!(error_message(502, "<html>"), "smbCloud answered 502.");
    }
}
