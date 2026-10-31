use {
    log::{debug, error},
    reqwest::{RequestBuilder, Response, StatusCode},
    serde::de::DeserializeOwned,
    smbcloud_model::{
        account::SmbAuthorization,
        error_codes::{ErrorCode, ErrorResponse},
        login::AccountStatus,
    },
};

//use std::time::Duration;
#[cfg(debug_assertions)]
const LOG_RESPONSE_BODY: bool = true; // You know what to do here.
#[cfg(not(debug_assertions))]
const LOG_RESPONSE_BODY: bool = false;

/// Check if there is an active internet connection.
///
/// Deprecated: this pings a third-party host, which fails behind an egress
/// allowlist even when the smbCloud API is reachable. A request that cannot
/// connect already returns `ErrorCode::NetworkError`.
#[cfg(not(target_arch = "wasm32"))]
#[deprecated(
    since = "1.1.0",
    note = "the request itself reports NetworkError; this pinged a third-party host"
)]
pub async fn check_internet_connection() -> bool {
    debug!("Checking internet connection");
    let client = reqwest::Client::builder()
        //.timeout(Duration::from_secs(5)) Does not work on wasm32
        .build();

    if let Err(e) = client {
        error!("Failed to create client for connectivity check: {:?}", e);
        return false;
    }

    if let Ok(client) = client {
        match client.get("https://dns.google").send().await {
            Ok(response) => {
                debug!(
                    "Internet connection check successful: {}",
                    response.status()
                );
                response.status().is_success()
            }
            Err(e) => {
                error!("Internet connection check failed: {:?}", e);
                false
            }
        }
    } else {
        false
    }
}

#[cfg(target_arch = "wasm32")]
#[deprecated(
    since = "1.1.0",
    note = "the request itself reports NetworkError; this pinged a third-party host"
)]
pub async fn check_internet_connection() -> bool {
    true
}

/// Log why a request failed to send (connect, timeout, proxy or other), so
/// `RUST_LOG=debug` explains a `NetworkError` without changing its message.
pub fn log_send_error(e: &reqwest::Error) {
    #[cfg(not(target_arch = "wasm32"))]
    let connect = e.is_connect();
    #[cfg(target_arch = "wasm32")]
    let connect = false;
    let kind = if e.is_timeout() {
        "timeout"
    } else if connect {
        "connect"
    } else if e.is_builder() {
        "builder"
    } else {
        "other"
    };
    debug!("Request failed to send ({kind}): {e:?}");
}

pub async fn parse_error_response<T: DeserializeOwned>(
    response: Response,
) -> Result<T, ErrorResponse> {
    let error_response_body = match response.text().await {
        Ok(body) => body,
        Err(e) => {
            error!("Failed to get response body: {:?}", e);
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::NetworkError,
                message: ErrorCode::NetworkError.message(None).to_string(),
            });
        }
    };

    if LOG_RESPONSE_BODY {
        debug!(
            "Parse Error >>>> {:?}",
            serde_json::to_string_pretty(&error_response_body)
        );
    }

    let error_response = match serde_json::from_str(&error_response_body) {
        Ok(error_response) => error_response,
        Err(e) => {
            error!("Failed to parse error response: {:?}", e);
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::ParseError,
                message: ErrorCode::ParseError.message(None).to_string(),
            });
        }
    };
    // The parsing itself is succeed.
    // Why is this an ErrorResponse.
    Err(error_response)
}

pub async fn request_login(builder: RequestBuilder) -> Result<AccountStatus, ErrorResponse> {
    let response = builder.send().await;
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            error!("request_login: Failed to get response: {:?}", e);
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::NetworkError,
                message: ErrorCode::NetworkError.message(None).to_string(),
            });
        }
    };

    if LOG_RESPONSE_BODY {
        debug!("request_login: Parse >>>> {:?}", response.status());
    }

    match (response.status(), response.headers().get("Authorization")) {
        (StatusCode::OK, Some(token)) => {
            // Login successful, let's get the access token for real.
            let access_token = match token.to_str() {
                Ok(token) => token.to_string(),
                Err(_) => {
                    return Err(ErrorResponse::Error {
                        error_code: ErrorCode::NetworkError,
                        message: ErrorCode::NetworkError.message(None).to_string(),
                    });
                }
            };
            Ok(AccountStatus::Ready { access_token })
        }
        (StatusCode::OK, None) => {
            // Silent login from oauth. Need improvement.
            let error_response = match parse_error_response::<ErrorResponse>(response).await {
                Ok(error) => error,
                Err(_) => return Ok(AccountStatus::NotFound),
            };
            match error_response {
                ErrorResponse::Error {
                    error_code,
                    message: _,
                } => match error_code {
                    ErrorCode::EmailNotVerified => Ok(AccountStatus::Incomplete {
                        status: smbcloud_model::account::ErrorCode::EmailUnverified,
                    }),
                    ErrorCode::PasswordNotSet => Ok(AccountStatus::Incomplete {
                        status: smbcloud_model::account::ErrorCode::PasswordNotSet,
                    }),
                    ErrorCode::Unknown => Ok(AccountStatus::Ready {
                        access_token: "tokenization".to_string(),
                    }),
                    _ => Ok(AccountStatus::NotFound),
                },
            }
        }
        (StatusCode::NOT_FOUND, _) => {
            // Account not found
            Ok(AccountStatus::NotFound)
        }
        (StatusCode::UNPROCESSABLE_ENTITY, _) => {
            let body_text = match response.text().await {
                Ok(text) => text,
                Err(_) => {
                    return Err(ErrorResponse::Error {
                        error_code: ErrorCode::NetworkError,
                        message: ErrorCode::NetworkError.message(None).to_string(),
                    });
                }
            };

            let result: SmbAuthorization = match serde_json::from_str(&body_text) {
                Ok(res) => res,
                Err(_) => {
                    // Try generic JSON for a useful message.
                    let message = serde_json::from_str::<serde_json::Value>(&body_text)
                        .ok()
                        .and_then(|v| {
                            v.get("message")
                                .and_then(serde_json::Value::as_str)
                                .map(ToOwned::to_owned)
                        })
                        .unwrap_or_else(|| ErrorCode::NetworkError.message(None).to_string());
                    return Err(ErrorResponse::Error {
                        error_code: ErrorCode::NetworkError,
                        message,
                    });
                }
            };

            let error_code = match result.error_code {
                Some(code) => code,
                None => {
                    // No error_code but we have a message — return it as Incomplete with a
                    // generic unverified status so the client can display the message.
                    return Err(ErrorResponse::Error {
                        error_code: ErrorCode::NetworkError,
                        message: result.message,
                    });
                }
            };
            Ok(AccountStatus::Incomplete { status: error_code })
        }
        (StatusCode::UNAUTHORIZED, _) => {
            let body_text = match response.text().await {
                Ok(text) => text,
                Err(_) => {
                    return Err(ErrorResponse::Error {
                        error_code: ErrorCode::NetworkError,
                        message: ErrorCode::NetworkError.message(None).to_string(),
                    });
                }
            };

            // Try parsing as SmbAuthorization first.
            if let Ok(result) = serde_json::from_str::<SmbAuthorization>(&body_text) {
                let error_code = match result.error_code {
                    Some(code) => code,
                    None => {
                        // Response parsed but no error_code — surface the server's message.
                        return Err(ErrorResponse::Error {
                            error_code: ErrorCode::Unauthorized,
                            message: result.message,
                        });
                    }
                };
                return Err(ErrorResponse::Error {
                    error_code: ErrorCode::Unauthorized,
                    message: error_code.to_string(),
                });
            }

            // Fallback: try to extract message or error from a generic JSON object.
            if let Ok(generic) = serde_json::from_str::<serde_json::Value>(&body_text) {
                let extracted = generic
                    .get("message")
                    .or_else(|| generic.get("error"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Sign in failed.");
                return Err(ErrorResponse::Error {
                    error_code: ErrorCode::Unauthorized,
                    message: extracted.to_string(),
                });
            }

            Err(ErrorResponse::Error {
                error_code: ErrorCode::Unauthorized,
                message: "Sign in failed.".to_string(),
            })
        }
        (status, _) => parse_error_response(response)
            .await
            .map_err(|_| ErrorResponse::Error {
                error_code: ErrorCode::NetworkError,
                message: format!("Unexpected login response status: {}", status),
            }),
    }
}

pub async fn request<R: DeserializeOwned>(builder: RequestBuilder) -> Result<R, ErrorResponse> {
    let response = builder.send().await;
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to get response: {:?}", e);
            log_send_error(&e);
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::NetworkError,
                message: ErrorCode::NetworkError.message(None).to_string(),
            });
        }
    };
    let response = match response.status() {
        reqwest::StatusCode::OK | reqwest::StatusCode::CREATED => response,
        status => {
            error!("Failed to get response: {:?}", status);
            // This should handle parsing the error response.
            return parse_error_response(response).await;
        }
    };

    let response_body = match response.text().await {
        Ok(body) => body,
        Err(e) => {
            error!("Failed to get response body: {:?}", e);
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::NetworkError,
                message: ErrorCode::NetworkError.message(None).to_string(),
            });
        }
    };

    if LOG_RESPONSE_BODY {
        debug!(
            "Parse >>>> {:?}",
            serde_json::to_string_pretty(&response_body)
        );
    }

    let response = match serde_json::from_str::<R>(&response_body) {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to parse response: {:?}", e);
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::ParseError,
                message: e.to_string(),
            });
        }
    };

    Ok(response)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn unreachable_host_reports_network_error_quickly() {
        let started = Instant::now();
        let builder = reqwest::Client::new().get("http://127.0.0.1:1");
        let result = request::<Value>(builder).await;
        match result {
            Err(ErrorResponse::Error { error_code, .. }) => {
                assert!(matches!(error_code, ErrorCode::NetworkError));
            }
            other => panic!("expected NetworkError, got {other:?}"),
        }
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn workspace_does_not_call_the_connectivity_preflight() {
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut callers = Vec::new();
        let mut stack = vec![crates];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.ends_with("smbcloud-network/src/network.rs")
                    && std::fs::read_to_string(&path)
                        .unwrap()
                        .contains("check_internet_connection")
                {
                    callers.push(path);
                }
            }
        }
        assert!(
            callers.is_empty(),
            "preflight still called from {callers:?}"
        );
    }
}
