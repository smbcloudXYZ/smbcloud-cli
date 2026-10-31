use reqwest::{RequestBuilder, StatusCode};
use serde_json::Value;
use smbcloud_model::error_codes::{ErrorCode, ErrorResponse};
use smbcloud_network::network::{log_send_error, parse_error_response};

pub(crate) async fn request_empty(builder: RequestBuilder) -> Result<(), ErrorResponse> {
    let response = builder.send().await.map_err(|e| {
        log_send_error(&e);
        ErrorResponse::Error {
            error_code: ErrorCode::NetworkError,
            message: ErrorCode::NetworkError.message(None).to_string(),
        }
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
