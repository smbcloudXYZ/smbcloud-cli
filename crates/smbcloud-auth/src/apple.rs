use {
    crate::client_credentials::{ClientCredentials, base_url_builder as tenant_base_url_builder},
    reqwest::Url,
    serde::{Deserialize, Serialize},
    smbcloud_model::error_codes::{ErrorCode, ErrorResponse},
    smbcloud_network::environment::Environment,
    uuid::Uuid,
};

const TENANT_APPLE_AUTHORIZE_PATH: &str = "v1/client/oauth/apple/authorize";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleAuthorizationRequest {
    pub authorize_url: String,
    pub redirect_uri: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleAuthSession {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub provider: String,
    pub provider_account_id: String,
    pub state: Option<String>,
}

pub fn build_authorization_request_with_client(
    env: Environment,
    client: ClientCredentials<'_>,
    redirect_uri: String,
    state: Option<String>,
) -> Result<AppleAuthorizationRequest, ErrorResponse> {
    let state = state.unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut url_builder = tenant_base_url_builder(env, client);

    url_builder
        .add_route(TENANT_APPLE_AUTHORIZE_PATH)
        .add_param("redirect_uri", &redirect_uri)
        .add_param("state", &state);

    Ok(AppleAuthorizationRequest {
        authorize_url: url_builder.build(),
        redirect_uri,
        state,
    })
}

pub fn parse_callback_url(
    callback_url: &str,
    expected_state: Option<&str>,
) -> Result<AppleAuthSession, ErrorResponse> {
    let url = Url::parse(callback_url).map_err(|err| ErrorResponse::Error {
        error_code: ErrorCode::ParseError,
        message: err.to_string(),
    })?;

    let mut access_token = None;
    let mut refresh_token = None;
    let mut email = None;
    let mut name = None;
    let mut provider = None;
    let mut provider_account_id = None;
    let mut state = None;
    let mut error = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "access_token" => access_token = Some(value.into_owned()),
            "refresh_token" => refresh_token = Some(value.into_owned()),
            "email" => email = Some(value.into_owned()),
            "name" => name = Some(value.into_owned()),
            "provider" => provider = Some(value.into_owned()),
            "provider_account_id" => provider_account_id = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            _ => {}
        }
    }

    if let Some(error) = error {
        return Err(ErrorResponse::Error {
            error_code: ErrorCode::InvalidParams,
            message: error,
        });
    }

    if let Some(expected_state) = expected_state {
        if state.as_deref() != Some(expected_state) {
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::InvalidParams,
                message: "Apple callback state mismatch.".to_string(),
            });
        }
    }

    Ok(AppleAuthSession {
        access_token: required_param("access_token", access_token)?,
        refresh_token,
        email,
        name,
        provider: provider.unwrap_or_else(|| "apple".to_string()),
        provider_account_id: required_param("provider_account_id", provider_account_id)?,
        state,
    })
}

fn required_param(
    name: &str,
    value: Option<String>,
) -> Result<String, ErrorResponse> {
    value.ok_or_else(|| ErrorResponse::Error {
        error_code: ErrorCode::InvalidParams,
        message: format!("Missing `{name}` in Apple callback URL."),
    })
}
