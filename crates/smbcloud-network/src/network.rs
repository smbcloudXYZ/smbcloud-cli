use {
    log::error,
    reqwest::{RequestBuilder, Response, StatusCode},
    serde::de::DeserializeOwned,
    smbcloud_model::{
        account::SmbAuthorization,
        error_codes::{ErrorCode, ErrorResponse},
        login::AccountStatus,
    },
};

#[cfg(not(target_arch = "wasm32"))]
use log::debug;

//use std::time::Duration;
#[cfg(debug_assertions)]
const LOG_RESPONSE_BODY: bool = true; // You know what to do here.
#[cfg(not(debug_assertions))]
const LOG_RESPONSE_BODY: bool = false;

/// Check if there is an active internet connection.
///
/// Native clients can afford a preflight connectivity check. Browser/wasm
/// clients should skip it and rely on the real request, otherwise the check
/// itself becomes a separate cross-origin failure surface.
#[cfg(not(target_arch = "wasm32"))]
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
pub async fn check_internet_connection() -> bool {
    true
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
        println!();
        println!("Parse Error >>>>");
        println!("{:?}", serde_json::to_string_pretty(&error_response_body));
        println!("Parse Error >>>>");
        println!();
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
        println!();
        println!("request_login: Parse >>>>");
        println!("{:?}", &response.status());
        println!("request_login: Parse >>>>");
        println!();
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
    // Check internet connection before making the request
    if !check_internet_connection().await {
        error!("No internet connection available");
        return Err(ErrorResponse::Error {
            error_code: ErrorCode::NetworkError,
            message: "No internet connection. Please check your network settings and try again."
                .to_string(),
        });
    }

    let response = builder.send().await;
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to get response: {:?}", e);
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
        println!();
        println!("Parse >>>>");
        println!("{:?}", serde_json::to_string_pretty(&response_body));
        println!("Parse >>>>");
        println!();
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
