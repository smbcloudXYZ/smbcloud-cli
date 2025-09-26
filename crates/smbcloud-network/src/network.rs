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
const LOG_RESPONSE_BODY: bool = false; // You know what to do here.
#[cfg(not(debug_assertions))]
const LOG_RESPONSE_BODY: bool = false;

/// Check if there is an active internet connection
///
/// This function attempts to connect to a reliable server (dns.google)
/// with a short timeout. Returns true if the connection was successful.
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

pub async fn parse_error_response<T: DeserializeOwned>(
    response: Response,
) -> Result<T, ErrorResponse> {
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
        println!("Parse Error >>>>");
        println!("{:?}", serde_json::to_string_pretty(&response_body));
        println!("Parse Error >>>>");
        println!();
    }

    let e = match serde_json::from_str::<ErrorResponse>(&response_body) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to parse error response: {:?}", e);
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::ParseError,
                message: ErrorCode::ParseError.message(None).to_string(),
            });
        }
    };
    error!("Error response: {:?}", e);
    Err(e)
}

pub async fn request_login(builder: RequestBuilder) -> Result<AccountStatus, ErrorResponse> {
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
        reqwest::StatusCode::OK
        | reqwest::StatusCode::NOT_FOUND
        | reqwest::StatusCode::UNPROCESSABLE_ENTITY => response,
        status => {
            error!(
                "Response are neither OK, NOT_FOUND, or UNPROCESSABLE_ENTITY: {:?}",
                status
            );
            return parse_error_response(response).await;
        }
    };

    if LOG_RESPONSE_BODY {
        println!();
        println!("Parse >>>>");
        println!("{:?}", &response.status());
        println!("Parse >>>>");
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
        (StatusCode::NOT_FOUND, _) => {
            // Account not found
            Ok(AccountStatus::NotFound)
        }
        (StatusCode::UNPROCESSABLE_ENTITY, _) => {
            // Account found but email not verified / password not set.
            let result: SmbAuthorization = match response.json().await {
                Ok(res) => res,
                Err(_) => {
                    return Err(ErrorResponse::Error {
                        error_code: ErrorCode::NetworkError,
                        message: ErrorCode::NetworkError.message(None).to_string(),
                    });
                }
            };
            // println!("Result: {:#?}", &result);
            let error_code = match result.error_code {
                Some(code) => code,
                None => {
                    return Err(ErrorResponse::Error {
                        error_code: ErrorCode::NetworkError,
                        message: ErrorCode::NetworkError.message(None).to_string(),
                    });
                }
            };
            Ok(AccountStatus::Incomplete { status: error_code })
        }
        _ => Err(ErrorResponse::Error {
            error_code: ErrorCode::NetworkError,
            message: ErrorCode::NetworkError.message(None).to_string(),
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
