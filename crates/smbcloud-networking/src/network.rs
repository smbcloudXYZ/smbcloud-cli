use log::{debug, error};
use reqwest::{RequestBuilder, Response};
use serde::de::DeserializeOwned;
use smbcloud_model::error_codes::{ErrorCode, ErrorResponse};
use std::time::Duration;
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
        .timeout(Duration::from_secs(5))
        .build();

    if let Err(e) = client {
        error!("Failed to create client for connectivity check: {:?}", e);
        return false;
    }

    match client.unwrap().get("https://dns.google").send().await {
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
