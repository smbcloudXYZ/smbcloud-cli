use log::error;
use reqwest::{RequestBuilder, Response};
use serde::de::DeserializeOwned;
use smbcloud_model::error_codes::{ErrorCode, ErrorResponse};
#[cfg(debug_assertions)]
const LOG_RESPONSE_BODY: bool = false; // You know what to do here.
#[cfg(not(debug_assertions))]
const LOG_RESPONSE_BODY: bool = false;

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
