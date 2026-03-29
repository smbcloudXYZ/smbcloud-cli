use magnus::{Error, Ruby, function, prelude::*};
use serde::Serialize;
use smbcloud_auth::{
    client_credentials::ClientCredentials, login::login_with_client, logout::logout_with_client,
    me::me_with_client, remove::remove_with_client, signup::signup_with_client,
};
use smbcloud_model::{error_codes::ErrorResponse, login::AccountStatus, signup::SignupResult};
use smbcloud_network::environment::Environment;
use tokio::runtime::Runtime;

#[derive(Serialize)]
struct LoginPayload {
    status: &'static str,
    access_token: Option<String>,
    error_code: Option<i32>,
    message: Option<String>,
}

fn parse_environment(value: String) -> Result<Environment, Error> {
    value.parse().map_err(|_| {
        Error::new(
            magnus::exception::arg_error(),
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
            magnus::exception::runtime_error(),
            format!("failed to initialize tokio runtime: {err}"),
        )
    })?;
    f(&runtime)
}

fn to_json<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|err| {
        Error::new(
            magnus::exception::runtime_error(),
            format!("failed to serialize response: {err}"),
        )
    })
}

fn raise_error_response(error: ErrorResponse) -> Error {
    let payload = serde_json::to_string(&error).unwrap_or_else(|_| {
        "{\"error\":{\"error_code\":0,\"message\":\"Unknown error.\"}}".to_string()
    });
    Error::new(magnus::exception::runtime_error(), payload)
}

fn login_payload(status: AccountStatus) -> LoginPayload {
    match status {
        AccountStatus::NotFound => LoginPayload {
            status: "not_found",
            access_token: None,
            error_code: None,
            message: None,
        },
        AccountStatus::Ready { access_token } => LoginPayload {
            status: "ready",
            access_token: Some(access_token),
            error_code: None,
            message: None,
        },
        AccountStatus::Incomplete { status } => LoginPayload {
            status: "incomplete",
            access_token: None,
            error_code: Some(status as i32),
            message: Some(status.to_string()),
        },
    }
}

fn credentials<'a>(app_id: &'a str, app_secret: &'a str) -> ClientCredentials<'a> {
    ClientCredentials { app_id, app_secret }
}

fn signup_with_client_json(
    environment: String,
    app_id: String,
    app_secret: String,
    email: String,
    password: String,
) -> Result<String, Error> {
    let env = parse_environment(environment)?;
    with_runtime(|runtime| {
        let result: Result<SignupResult, ErrorResponse> = runtime.block_on(signup_with_client(
            env,
            credentials(&app_id, &app_secret),
            email,
            password,
        ));
        match result {
            Ok(result) => to_json(&result),
            Err(error) => Err(raise_error_response(error)),
        }
    })
}

fn login_with_client_json(
    environment: String,
    app_id: String,
    app_secret: String,
    email: String,
    password: String,
) -> Result<String, Error> {
    let env = parse_environment(environment)?;
    with_runtime(|runtime| {
        let result: Result<AccountStatus, ErrorResponse> = runtime.block_on(login_with_client(
            env,
            credentials(&app_id, &app_secret),
            email,
            password,
        ));
        match result {
            Ok(result) => to_json(&login_payload(result)),
            Err(error) => Err(raise_error_response(error)),
        }
    })
}

fn me_with_client_json(
    environment: String,
    app_id: String,
    app_secret: String,
    access_token: String,
) -> Result<String, Error> {
    let env = parse_environment(environment)?;
    with_runtime(|runtime| {
        let result = runtime.block_on(me_with_client(
            env,
            credentials(&app_id, &app_secret),
            &access_token,
        ));
        match result {
            Ok(result) => to_json(&result),
            Err(error) => Err(raise_error_response(error)),
        }
    })
}

fn logout_with_client_json(
    environment: String,
    app_id: String,
    app_secret: String,
    access_token: String,
) -> Result<bool, Error> {
    let env = parse_environment(environment)?;
    with_runtime(|runtime| {
        let result = runtime.block_on(logout_with_client(
            env,
            credentials(&app_id, &app_secret),
            access_token,
        ));
        match result {
            Ok(()) => Ok(true),
            Err(error) => Err(raise_error_response(error)),
        }
    })
}

fn remove_with_client_json(
    environment: String,
    app_id: String,
    app_secret: String,
    access_token: String,
) -> Result<bool, Error> {
    let env = parse_environment(environment)?;
    with_runtime(|runtime| {
        let result = runtime.block_on(remove_with_client(
            env,
            credentials(&app_id, &app_secret),
            &access_token,
        ));
        match result {
            Ok(()) => Ok(true),
            Err(error) => Err(raise_error_response(error)),
        }
    })
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let smbcloud = ruby.define_module("SmbCloud")?;
    let auth = smbcloud.define_module("Auth")?;

    auth.define_singleton_method(
        "__signup_with_client",
        function!(signup_with_client_json, 5),
    )?;
    auth.define_singleton_method("__login_with_client", function!(login_with_client_json, 5))?;
    auth.define_singleton_method("__me_with_client", function!(me_with_client_json, 4))?;
    auth.define_singleton_method(
        "__logout_with_client",
        function!(logout_with_client_json, 4),
    )?;
    auth.define_singleton_method(
        "__remove_with_client",
        function!(remove_with_client_json, 4),
    )?;

    Ok(())
}
