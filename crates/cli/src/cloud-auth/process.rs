use crate::{
    cli::CommandResult,
    client,
    cloud_auth::{
        auth_app::{
            create_auth_app, delete_auth_app, get_auth_app, get_auth_apps, update_auth_app,
        },
        cli::Commands,
        render::{print_auth_app_detail, print_auth_apps},
    },
    mail::current_project::{resolve_optional_project_id, resolve_required_project_id},
    token::get_smb_token::get_smb_token,
    ui::{fail_message, fail_symbol, prompt::confirm_delete, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use smbcloud_model::app_auth::{AuthAppCreate, AuthAppUpdate};
use smbcloud_network::environment::Environment;
use spinners::Spinner;

pub async fn process_cloud_auth(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::List { project_id } => process_auth_app_list(env, project_id).await,
        Commands::Show { id } => process_auth_app_show(env, id).await,
        Commands::New {
            name,
            project_id,
            support_email,
        } => process_auth_app_new(env, name, project_id, support_email).await,
        Commands::Update {
            id,
            name,
            support_email,
        } => process_auth_app_update(env, id, name, support_email).await,
        Commands::Delete { id } => process_auth_app_delete(env, id).await,
    }
}

async fn process_auth_app_list(
    env: Environment,
    project_id: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let resolved_project_id = resolve_optional_project_id(env, normalize_optional(project_id))?;
    let mut spinner = loading_spinner("Loading Auth apps");

    let auth_apps = get_auth_apps(env, client(), access_token, resolved_project_id)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    print_auth_apps(&auth_apps);

    Ok(done_result(if auth_apps.is_empty() {
        "No Auth apps found."
    } else {
        "Done."
    }))
}

async fn process_auth_app_show(env: Environment, id: String) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let auth_app_id = normalize_required("Auth app id", id)?;
    let mut spinner = loading_spinner("Loading Auth app");

    let auth_app = get_auth_app(env, client(), access_token, auth_app_id)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    print_auth_app_detail(&auth_app);

    Ok(done_result("Done."))
}

async fn process_auth_app_new(
    env: Environment,
    name: String,
    project_id: Option<String>,
    support_email: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let auth_app = AuthAppCreate {
        name: normalize_required("name", name)?,
        project_id: resolve_required_project_id(env, normalize_optional(project_id))?,
        support_email: normalize_optional(support_email),
    };
    let mut spinner = loading_spinner("Creating Auth app");

    let auth_app = create_auth_app(env, client(), access_token, auth_app)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Created."));
    print_auth_app_detail(&auth_app);

    Ok(done_result("Auth app created."))
}

async fn process_auth_app_update(
    env: Environment,
    id: String,
    name: Option<String>,
    support_email: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let auth_app_id = normalize_required("Auth app id", id)?;
    let update = AuthAppUpdate {
        name: normalize_optional(name),
        support_email: normalize_optional(support_email),
    };

    if update.is_empty() {
        return Err(anyhow!(
            "Specify at least one of `--name` or `--support-email`."
        ));
    }

    let mut spinner = loading_spinner("Updating Auth app");
    let auth_app = update_auth_app(env, client(), access_token, auth_app_id, update)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Updated."));
    print_auth_app_detail(&auth_app);

    Ok(done_result("Auth app updated."))
}

async fn process_auth_app_delete(env: Environment, id: String) -> Result<CommandResult> {
    let auth_app_id = normalize_required("Auth app id", id)?;
    let confirmed = confirm_delete(
        "Auth app deletion confirmation",
        &format!("Delete Auth app #{auth_app_id}"),
    )?;

    if !confirmed {
        return Ok(done_result("Cancelled."));
    }

    let access_token = get_smb_token(env)?;
    let spinner = loading_spinner("Deleting Auth app");

    match delete_auth_app(env, client(), access_token, auth_app_id).await {
        Ok(()) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Done. Auth app has been deleted."),
        }),
        Err(error) => Ok(CommandResult {
            spinner,
            symbol: fail_symbol(),
            msg: fail_message(&error.to_string()),
        }),
    }
}

fn loading_spinner(message: &str) -> Spinner {
    Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message(message),
    )
}

fn done_result(message: &str) -> CommandResult {
    CommandResult {
        spinner: loading_spinner("Done"),
        symbol: succeed_symbol(),
        msg: succeed_message(message),
    }
}

fn normalize_required(label: &str, value: String) -> Result<String> {
    let normalized_value = value.trim().to_string();
    if normalized_value.is_empty() {
        return Err(anyhow!("{} cannot be empty.", label));
    }
    Ok(normalized_value)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let normalized_value = value.trim().to_string();
        if normalized_value.is_empty() {
            None
        } else {
            Some(normalized_value)
        }
    })
}

fn api_error(error: impl std::fmt::Display) -> anyhow::Error {
    anyhow!(error.to_string())
}
