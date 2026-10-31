use magnus::{function, prelude::*, Error, Ruby};
use smbcloud_email_sdk::{EmailClient, EmailCredentials, Environment, SendEmail};
use tokio::runtime::Runtime;

// Every helper here runs on the Ruby thread that called into the extension
// (tokio's `block_on` stays on the calling thread), so the handle is available.
fn ruby() -> Ruby {
    Ruby::get().expect("smbcloud-email called outside a Ruby thread")
}

fn parse_environment(value: String) -> Result<Environment, Error> {
    value.parse().map_err(|_| {
        Error::new(
            ruby().exception_arg_error(),
            format!("invalid environment `{value}`, expected `dev` or `production`"),
        )
    })
}

fn with_runtime<T, F>(f: F) -> Result<T, Error>
where
    F: FnOnce(&Runtime) -> Result<T, Error>,
{
    let runtime = Runtime::new().map_err(|err| {
        Error::new(
            ruby().exception_runtime_error(),
            format!("failed to initialize tokio runtime: {err}"),
        )
    })?;
    f(&runtime)
}

fn runtime_error(message: impl std::fmt::Display) -> Error {
    Error::new(ruby().exception_runtime_error(), message.to_string())
}

fn client(environment: Environment, api_key: &str) -> EmailClient {
    EmailClient::from_credentials(environment, EmailCredentials { api_key })
}

/// `message_json` is the SendEmail payload built on the Ruby side. Returns the
/// created message as JSON.
fn send_json(environment: String, api_key: String, message_json: String) -> Result<String, Error> {
    let env = parse_environment(environment)?;
    let message: SendEmail = serde_json::from_str(&message_json)
        .map_err(|err| Error::new(ruby().exception_arg_error(), format!("invalid message: {err}")))?;

    with_runtime(|runtime| {
        match runtime.block_on(client(env, &api_key).send(&message)) {
            Ok(result) => serde_json::to_string(&result).map_err(runtime_error),
            Err(error) => Err(runtime_error(error)),
        }
    })
}

fn get_message_json(environment: String, api_key: String, id: String) -> Result<String, Error> {
    let env = parse_environment(environment)?;
    with_runtime(|runtime| {
        match runtime.block_on(client(env, &api_key).get_message(&id)) {
            Ok(result) => serde_json::to_string(&result).map_err(runtime_error),
            Err(error) => Err(runtime_error(error)),
        }
    })
}

/// `status` empty string means "no filter"; `limit` <= 0 means "server default".
fn list_messages_json(
    environment: String,
    api_key: String,
    status: String,
    limit: i64,
) -> Result<String, Error> {
    let env = parse_environment(environment)?;
    let status = if status.trim().is_empty() { None } else { Some(status) };
    let limit = if limit > 0 { Some(limit as u32) } else { None };

    with_runtime(|runtime| {
        match runtime.block_on(client(env, &api_key).list_messages(status.as_deref(), limit)) {
            Ok(result) => serde_json::to_string(&result).map_err(runtime_error),
            Err(error) => Err(runtime_error(error)),
        }
    })
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let smbcloud = ruby.define_module("SmbCloud")?;
    let email = smbcloud.define_module("Email")?;

    email.define_singleton_method("__send", function!(send_json, 3))?;
    email.define_singleton_method("__get_message", function!(get_message_json, 3))?;
    email.define_singleton_method("__list_messages", function!(list_messages_json, 4))?;

    Ok(())
}
