#![allow(unexpected_cfgs, unsafe_op_in_unsafe_fn)]

use pyo3::{
    create_exception,
    exceptions::{PyException, PyValueError},
    prelude::*,
};
use pythonize::pythonize;
use serde::Serialize;
use smbcloud_auth::{
    client_credentials::ClientCredentials, login::login_with_client as rust_login_with_client,
    logout::logout_with_client as rust_logout_with_client,
    me::me_with_client as rust_me_with_client,
    remove::remove_with_client as rust_remove_with_client,
    signup::signup_with_client as rust_signup_with_client,
};
use smbcloud_model::{
    account::ErrorCode as AccountErrorCode, error_codes::ErrorResponse, login::AccountStatus,
};
use smbcloud_network::environment::Environment;
use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

create_exception!(_native, NativeSdkError, PyException);

#[derive(Serialize)]
struct ErrorPayload {
    error_code: i32,
    error_name: String,
    message: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum LoginPayload {
    NotFound,
    Ready {
        access_token: String,
    },
    Incomplete {
        status: String,
        status_code: u32,
        message: String,
    },
}

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to initialize tokio runtime for smbcloud auth python sdk")
    })
}

fn parse_env(env: &str) -> PyResult<Environment> {
    env.parse()
        .map_err(|_| PyValueError::new_err("env must be 'dev' or 'production'"))
}

fn client_credentials<'a>(app_id: &'a str, app_secret: &'a str) -> ClientCredentials<'a> {
    ClientCredentials { app_id, app_secret }
}

fn serialize<'py, T>(py: Python<'py>, value: &T) -> PyResult<PyObject>
where
    T: Serialize,
{
    Ok(pythonize(py, value)?.unbind().into())
}

fn login_payload(status: AccountStatus) -> LoginPayload {
    match status {
        AccountStatus::NotFound => LoginPayload::NotFound,
        AccountStatus::Ready { access_token } => LoginPayload::Ready { access_token },
        AccountStatus::Incomplete { status } => LoginPayload::Incomplete {
            status: account_error_name(status).to_string(),
            status_code: status as u32,
            message: status.to_string(),
        },
    }
}

fn account_error_name(code: AccountErrorCode) -> &'static str {
    match code {
        AccountErrorCode::EmailNotFound => "email_not_found",
        AccountErrorCode::EmailUnverified => "email_unverified",
        AccountErrorCode::PasswordNotSet => "password_not_set",
        AccountErrorCode::GithubNotLinked => "github_not_linked",
    }
}

fn native_error(error: ErrorResponse) -> PyErr {
    let ErrorResponse::Error {
        error_code,
        message,
    } = error;

    let error_name = error_code.rb_constant_name();
    let payload = ErrorPayload {
        error_code: error_code as i32,
        error_name,
        message,
    };

    let message = serde_json::to_string(&payload).unwrap_or_else(|_| {
        "{\"error_code\":0,\"error_name\":\"Unknown\",\"message\":\"Unknown error.\"}".to_string()
    });

    NativeSdkError::new_err(message)
}

#[pyfunction]
fn signup_with_client(
    py: Python<'_>,
    env: &str,
    app_id: &str,
    app_secret: &str,
    email: &str,
    password: &str,
) -> PyResult<PyObject> {
    let env = parse_env(env)?;
    let client = client_credentials(app_id, app_secret);
    let result = runtime()
        .block_on(rust_signup_with_client(
            env,
            client,
            email.to_string(),
            password.to_string(),
        ))
        .map_err(native_error)?;

    serialize(py, &result)
}

#[pyfunction]
fn login_with_client(
    py: Python<'_>,
    env: &str,
    app_id: &str,
    app_secret: &str,
    email: &str,
    password: &str,
) -> PyResult<PyObject> {
    let env = parse_env(env)?;
    let client = client_credentials(app_id, app_secret);
    let result = runtime()
        .block_on(rust_login_with_client(
            env,
            client,
            email.to_string(),
            password.to_string(),
        ))
        .map(login_payload)
        .map_err(native_error)?;

    serialize(py, &result)
}

#[pyfunction]
fn logout_with_client(
    env: &str,
    app_id: &str,
    app_secret: &str,
    access_token: &str,
) -> PyResult<()> {
    let env = parse_env(env)?;
    let client = client_credentials(app_id, app_secret);

    runtime()
        .block_on(rust_logout_with_client(
            env,
            client,
            access_token.to_string(),
        ))
        .map_err(native_error)
}

#[pyfunction]
fn me_with_client(
    py: Python<'_>,
    env: &str,
    app_id: &str,
    app_secret: &str,
    access_token: &str,
) -> PyResult<PyObject> {
    let env = parse_env(env)?;
    let client = client_credentials(app_id, app_secret);
    let result = runtime()
        .block_on(rust_me_with_client(env, client, access_token))
        .map_err(native_error)?;

    serialize(py, &result)
}

#[pyfunction]
fn remove_with_client(
    env: &str,
    app_id: &str,
    app_secret: &str,
    access_token: &str,
) -> PyResult<()> {
    let env = parse_env(env)?;
    let client = client_credentials(app_id, app_secret);

    runtime()
        .block_on(rust_remove_with_client(env, client, access_token))
        .map_err(native_error)
}

#[pymodule(name = "_native")]
fn python_module(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("NativeSdkError", py.get_type_bound::<NativeSdkError>())?;
    module.add_function(wrap_pyfunction!(signup_with_client, module)?)?;
    module.add_function(wrap_pyfunction!(login_with_client, module)?)?;
    module.add_function(wrap_pyfunction!(logout_with_client, module)?)?;
    module.add_function(wrap_pyfunction!(me_with_client, module)?)?;
    module.add_function(wrap_pyfunction!(remove_with_client, module)?)?;
    Ok(())
}
