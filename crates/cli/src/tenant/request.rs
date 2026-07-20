use reqwest::{RequestBuilder, StatusCode};
use serde_json::Value;
use smbcloud_model::error_codes::{ErrorCode, ErrorResponse};
use smbcloud_network::network::{check_internet_connection, parse_error_response};

pub(crate) async fn request_empty(builder: RequestBuilder) -> Result<(), ErrorResponse> {
    if !check_internet_connection().await {
        return Err(ErrorResponse::Error {
            error_code: ErrorCode::NetworkError,
            message: "No internet connection. Please check your network settings and try again."
                .to_string(),
        });
    }

    let response = builder.send().await.map_err(|_| ErrorResponse::Error {
        error_code: ErrorCode::NetworkError,
        message: ErrorCode::NetworkError.message(None).to_string(),
    })?;

    match response.status() {
        StatusCode::OK | StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(()),
        _ => match parse_error_response::<Value>(response).await {
            Ok(_) => Err(ErrorResponse::Error {
                error_code: ErrorCode::ParseError,
                message: ErrorCode::ParseError.message(None).to_string(),
            }),
            Err(error_response) => Err(error_response),
        },
    }
}
