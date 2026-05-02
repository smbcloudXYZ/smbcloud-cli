use crate::{
    cli::CommandResult,
    client,
    mail::{
        cli::{Commands, InboxCommands, MessageCommands},
        current_project::{resolve_optional_project_id, resolve_required_project_id},
        render::{
            print_mail_app_detail, print_mail_apps, print_mail_inbox_detail,
            print_mail_message_detail, print_mail_messages, print_mail_test_delivery,
        },
    },
    token::get_smb_token::get_smb_token,
    ui::{
        confirm_dialog::confirm_delete_tui, fail_message, fail_symbol, succeed_message,
        succeed_symbol,
    },
};
use anyhow::{anyhow, Result};
use smbcloud_mail::{
    mail_app::{create_mail_app, delete_mail_app, get_mail_app, get_mail_apps, update_mail_app},
    mail_inbox::{create_mail_inbox, delete_mail_inbox, send_test_email, update_mail_inbox},
    mail_message::{get_mail_message, get_mail_messages},
};
use smbcloud_model::mail::{
    MailAppCreate, MailAppUpdate, MailInboxCreate, MailInboxUpdate, MailTestEmailRequest,
};
use smbcloud_network::environment::Environment;
use spinners::Spinner;

pub async fn process_mail(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::List { project_id } => process_mail_list(env, project_id).await,
        Commands::Show { id } => process_mail_show(env, id).await,
        Commands::New {
            name,
            domain,
            project_id,
            aws_region,
        } => process_mail_new(env, name, domain, project_id, aws_region).await,
        Commands::Update {
            id,
            name,
            domain,
            aws_region,
        } => process_mail_update(env, id, name, domain, aws_region).await,
        Commands::Delete { id } => process_mail_delete(env, id).await,
        Commands::Inbox { command } => process_mail_inbox(env, command).await,
        Commands::Message { command } => process_mail_message(env, command).await,
    }
}

async fn process_mail_inbox(env: Environment, commands: InboxCommands) -> Result<CommandResult> {
    match commands {
        InboxCommands::New {
            app_id,
            local_part,
            forward_to,
            sender_email,
        } => process_mail_inbox_new(env, app_id, local_part, forward_to, sender_email).await,
        InboxCommands::Update {
            app_id,
            id,
            local_part,
            forward_to,
            sender_email,
        } => process_mail_inbox_update(env, app_id, id, local_part, forward_to, sender_email).await,
        InboxCommands::Delete { app_id, id } => process_mail_inbox_delete(env, app_id, id).await,
        InboxCommands::Test {
            app_id,
            id,
            recipient_email,
            subject,
            body,
        } => process_mail_inbox_test(env, app_id, id, recipient_email, subject, body).await,
    }
}

async fn process_mail_message(
    env: Environment,
    commands: MessageCommands,
) -> Result<CommandResult> {
    match commands {
        MessageCommands::List {
            app_id,
            inbox_id,
            limit,
        } => process_mail_message_list(env, app_id, inbox_id, limit).await,
        MessageCommands::Show {
            app_id,
            inbox_id,
            id,
        } => process_mail_message_show(env, app_id, inbox_id, id).await,
    }
}

async fn process_mail_list(env: Environment, project_id: Option<String>) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let resolved_project_id = resolve_optional_project_id(env, normalize_optional(project_id))?;
    let mut spinner = loading_spinner("Loading mail apps");

    let mail_apps = get_mail_apps(env, client(), access_token, resolved_project_id)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    print_mail_apps(&mail_apps);

    Ok(done_result(if mail_apps.is_empty() {
        "No mail apps found."
    } else {
        "Done."
    }))
}

async fn process_mail_show(env: Environment, id: String) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mail_app_id = normalize_required("mail app id", id)?;
    let mut spinner = loading_spinner("Loading mail app");

    let mail_app = get_mail_app(env, client(), access_token, mail_app_id)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    print_mail_app_detail(&mail_app);

    Ok(done_result("Done."))
}

async fn process_mail_new(
    env: Environment,
    name: String,
    domain: String,
    project_id: Option<String>,
    aws_region: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mail_app = MailAppCreate {
        name: normalize_required("name", name)?,
        project_id: resolve_required_project_id(env, normalize_optional(project_id))?,
        domain: normalize_required("domain", domain)?,
        aws_region: normalize_optional(aws_region),
    };
    let mut spinner = loading_spinner("Creating mail app");

    let mail_app = create_mail_app(env, client(), access_token, mail_app)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Created."));
    print_mail_app_detail(&mail_app);

    Ok(done_result("Mail app created."))
}

async fn process_mail_update(
    env: Environment,
    id: String,
    name: Option<String>,
    domain: Option<String>,
    aws_region: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mail_app_id = normalize_required("mail app id", id)?;
    let update = MailAppUpdate {
        name: normalize_optional(name),
        domain: normalize_optional(domain),
        aws_region: normalize_optional(aws_region),
    };

    if update.is_empty() {
        return Err(anyhow!(
            "Specify at least one of `--name`, `--domain`, or `--aws-region`."
        ));
    }

    let mut spinner = loading_spinner("Updating mail app");
    let mail_app = update_mail_app(env, client(), access_token, mail_app_id, update)
        .await
        .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Updated."));
    print_mail_app_detail(&mail_app);

    Ok(done_result("Mail app updated."))
}

async fn process_mail_delete(env: Environment, id: String) -> Result<CommandResult> {
    let mail_app_id = normalize_required("mail app id", id)?;
    let confirmed = confirm_delete_tui(&format!("Delete mail app #{mail_app_id}"))
        .map_err(|error| anyhow!(error))?;

    if !confirmed {
        return Ok(done_result("Cancelled."));
    }

    let access_token = get_smb_token(env)?;
    let spinner = loading_spinner("Deleting mail app");

    match delete_mail_app(env, client(), access_token, mail_app_id).await {
        Ok(()) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Done. Mail app has been deleted."),
        }),
        Err(error) => Ok(CommandResult {
            spinner,
            symbol: fail_symbol(),
            msg: fail_message(&error.to_string()),
        }),
    }
}

async fn process_mail_inbox_new(
    env: Environment,
    app_id: String,
    local_part: String,
    forward_to: String,
    sender_email: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mail_inbox = MailInboxCreate {
        local_part: normalize_required("local part", local_part)?,
        forward_to_email: normalize_required("forward target", forward_to)?,
        sender_email: normalize_optional(sender_email),
    };
    let mut spinner = loading_spinner("Creating mail inbox");

    let mail_inbox = create_mail_inbox(
        env,
        client(),
        access_token,
        normalize_required("mail app id", app_id)?,
        mail_inbox,
    )
    .await
    .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Created."));
    print_mail_inbox_detail(&mail_inbox);

    Ok(done_result("Mail inbox created."))
}

async fn process_mail_inbox_update(
    env: Environment,
    app_id: String,
    id: String,
    local_part: Option<String>,
    forward_to: Option<String>,
    sender_email: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let update = MailInboxUpdate {
        local_part: normalize_optional(local_part),
        forward_to_email: normalize_optional(forward_to),
        sender_email: normalize_optional(sender_email),
    };

    if update.is_empty() {
        return Err(anyhow!(
            "Specify at least one of `--local-part`, `--forward-to`, or `--sender-email`."
        ));
    }

    let mut spinner = loading_spinner("Updating mail inbox");
    let mail_inbox = update_mail_inbox(
        env,
        client(),
        access_token,
        normalize_required("mail app id", app_id)?,
        normalize_required("mail inbox id", id)?,
        update,
    )
    .await
    .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Updated."));
    print_mail_inbox_detail(&mail_inbox);

    Ok(done_result("Mail inbox updated."))
}

async fn process_mail_inbox_delete(
    env: Environment,
    app_id: String,
    id: String,
) -> Result<CommandResult> {
    let mail_app_id = normalize_required("mail app id", app_id)?;
    let mail_inbox_id = normalize_required("mail inbox id", id)?;
    let confirmed = confirm_delete_tui(&format!(
        "Delete mail inbox #{mail_inbox_id} from mail app #{mail_app_id}"
    ))
    .map_err(|error| anyhow!(error))?;

    if !confirmed {
        return Ok(done_result("Cancelled."));
    }

    let access_token = get_smb_token(env)?;
    let spinner = loading_spinner("Deleting mail inbox");

    match delete_mail_inbox(env, client(), access_token, mail_app_id, mail_inbox_id).await {
        Ok(()) => Ok(CommandResult {
            spinner,
            symbol: succeed_symbol(),
            msg: succeed_message("Done. Mail inbox has been deleted."),
        }),
        Err(error) => Ok(CommandResult {
            spinner,
            symbol: fail_symbol(),
            msg: fail_message(&error.to_string()),
        }),
    }
}

async fn process_mail_inbox_test(
    env: Environment,
    app_id: String,
    id: String,
    recipient_email: Option<String>,
    subject: Option<String>,
    body: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let request = MailTestEmailRequest {
        recipient_email: normalize_optional(recipient_email),
        subject: normalize_optional(subject),
        body: normalize_optional(body),
    };
    let mut spinner = loading_spinner("Sending mail test email");

    let delivery = send_test_email(
        env,
        client(),
        access_token,
        normalize_required("mail app id", app_id)?,
        normalize_required("mail inbox id", id)?,
        request,
    )
    .await
    .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Sent."));
    print_mail_test_delivery(&delivery);

    Ok(done_result("Mail test email sent."))
}

async fn process_mail_message_list(
    env: Environment,
    app_id: String,
    inbox_id: String,
    limit: u32,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mut spinner = loading_spinner("Loading mail messages");

    let messages = get_mail_messages(
        env,
        client(),
        access_token,
        normalize_required("mail app id", app_id)?,
        normalize_required("mail inbox id", inbox_id)?,
        Some(limit.clamp(1, 100)),
    )
    .await
    .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    print_mail_messages(&messages);

    Ok(done_result(if messages.is_empty() {
        "No mail messages found."
    } else {
        "Done."
    }))
}

async fn process_mail_message_show(
    env: Environment,
    app_id: String,
    inbox_id: String,
    id: String,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let mut spinner = loading_spinner("Loading mail message");

    let message = get_mail_message(
        env,
        client(),
        access_token,
        normalize_required("mail app id", app_id)?,
        normalize_required("mail inbox id", inbox_id)?,
        normalize_required("mail message id", id)?,
    )
    .await
    .map_err(api_error)?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));
    print_mail_message_detail(&message);

    Ok(done_result("Done."))
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
