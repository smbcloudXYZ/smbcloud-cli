use crate::{
    cli::CommandResult,
    client,
    tenant::{
        cli::Commands,
        render::{print_tenant_detail, print_tenants},
        tenant_client::{create_tenant, delete_tenant, get_tenant, get_tenants, update_tenant},
    },
    token::get_smb_token::get_smb_token,
    ui::{fail_message, fail_symbol, prompt::confirm_delete_double, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use smbcloud_model::tenant::{TenantCreate, TenantKind, TenantUpdate};
use smbcloud_network::environment::Environment;
use spinners::Spinner;

pub async fn process_tenant(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::List {} => process_tenant_list(env).await,
        Commands::Show { id } => process_tenant_show(env, id).await,
        Commands::New { name } => process_tenant_new(env, name).await,
        Commands::Update { id, name } => process_tenant_update(env, id, name).await,
        Commands::Delete { id } => process_tenant_delete(env, id).await,
    }
}

async fn process_tenant_list(env: Environment) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mut spinner = loading_spinner("Loading tenants");

    let tenants = get_tenants(env, client(), access_token)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    print_tenants(&tenants);

    Ok(done_result(if tenants.is_empty() {
        "No tenants found."
    } else {
        "Done."
    }))
}

async fn process_tenant_show(env: Environment, id: String) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let tenant_id = normalize_required("Tenant id", id)?;
    let mut spinner = loading_spinner("Loading tenant");

    let tenant = get_tenant(env, client(), access_token, tenant_id)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    print_tenant_detail(&tenant);

    Ok(done_result("Done."))
}

async fn process_tenant_new(env: Environment, name: String) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let tenant = TenantCreate {
        name: normalize_required("name", name)?,
    };
    let mut spinner = loading_spinner("Creating tenant");

    let tenant = create_tenant(env, client(), access_token, tenant)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Created."));
    print_tenant_detail(&tenant);

    Ok(done_result("Tenant created."))
}

async fn process_tenant_update(env: Environment, id: String, name: String) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let tenant_id = normalize_required("Tenant id", id)?;
    let update = TenantUpdate {
        name: normalize_optional(Some(name)),
    };

    if update.is_empty() {
        return Err(anyhow!("Specify `--name`."));
    }

    let mut spinner = loading_spinner("Updating tenant");
    let tenant = update_tenant(env, client(), access_token, tenant_id, update)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Updated."));
    print_tenant_detail(&tenant);

    Ok(done_result("Tenant updated."))
}

async fn process_tenant_delete(env: Environment, id: String) -> Result<CommandResult> {
    let tenant_id = normalize_required("Tenant id", id)?;
    let access_token = get_smb_token(env)?;

    let tenant = get_tenant(env, client(), access_token.clone(), tenant_id.clone())
        .await
        .map_err(api_error)?;

    // The personal tenant is bootstrapped on signup and isn't a
    // user-manageable resource — mirror the API-side guard client-side so
    // the failure is immediate instead of after two confirmation prompts.
    if matches!(tenant.kind, TenantKind::Personal) {
        return Err(anyhow!("The personal tenant can't be deleted."));
    }

    let warning = format!(
        "This permanently deletes \"{}\" and everything it owns — {} project(s), plus its auth apps, mail apps, and domains.",
        tenant.name, tenant.projects_count
    );
    let confirmed = confirm_delete_double(
        "Tenant deletion confirmation",
        &warning,
        &tenant.slug,
        "delete my tenant",
    )?;

    if !confirmed {
        return Ok(done_result("Cancelled."));
    }

    let spinner = loading_spinner("Deleting tenant");

    match delete_tenant(env, client(), access_token, tenant_id).await {
        Ok(()) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Done. Tenant has been deleted."),
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
