use {
    base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _},
    openssl::sha::sha256,
    reqwest::{Client, Url},
    serde::{Deserialize, Serialize},
    smbcloud_model::error_codes::{ErrorCode, ErrorResponse},
    smbcloud_network::{environment::Environment, network},
    uuid::Uuid,
};

const AUTHORIZE_PATH: &str = "oauth/authorize";
const TOKEN_PATH: &str = "oauth/token";
const USERINFO_PATH: &str = "oauth/userinfo";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    pub authorize_url: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_verifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackPayload {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i32>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub tenant_id: Option<u64>,
    pub tenant_slug: Option<String>,
}

pub fn build_authorization_request(
    env: Environment,
    oidc_client_id: &str,
    redirect_uri: String,
) -> Result<AuthorizationRequest, ErrorResponse> {
    let code_verifier = format!(
        "{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let code_challenge = URL_SAFE_NO_PAD.encode(sha256(code_verifier.as_bytes()));
    let state = Uuid::new_v4().to_string();

    let mut url = issuer_base_url(env).join(AUTHORIZE_PATH).map_err(|err| ErrorResponse::Error {
        error_code: ErrorCode::ParseError,
        message: err.to_string(),
    })?;

    url.query_pairs_mut()
        .append_pair("client_id", oidc_client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid profile email")
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256");

    Ok(AuthorizationRequest {
        authorize_url: url.to_string(),
        redirect_uri,
        state,
        code_verifier,
    })
}

pub fn parse_callback_url(callback_url: &str) -> Result<CallbackPayload, ErrorResponse> {
    let url = Url::parse(callback_url).map_err(|err| ErrorResponse::Error {
        error_code: ErrorCode::ParseError,
        message: err.to_string(),
    })?;

    let mut code = None;
    let mut state = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }

    match (code, state) {
        (Some(code), Some(state)) => Ok(CallbackPayload { code, state }),
        _ => Err(ErrorResponse::Error {
            error_code: ErrorCode::InvalidParams,
            message: "Missing authorization code or state.".to_string(),
        }),
    }
}

pub async fn exchange_code(
    env: Environment,
    oidc_client_id: &str,
    redirect_uri: &str,
    code: &str,
    code_verifier: &str,
) -> Result<TokenResponse, ErrorResponse> {
    let url = issuer_base_url(env).join(TOKEN_PATH).map_err(|err| ErrorResponse::Error {
        error_code: ErrorCode::ParseError,
        message: err.to_string(),
    })?;

    let builder = Client::new()
        .post(url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", oidc_client_id),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ])
        .header("Accept", "application/json");

    network::request(builder).await
}

pub async fn get_userinfo(
    env: Environment,
    access_token: &str,
    tenant_id: Option<&str>,
) -> Result<UserInfo, ErrorResponse> {
    let url = issuer_base_url(env)
        .join(USERINFO_PATH)
        .map_err(|err| ErrorResponse::Error {
            error_code: ErrorCode::ParseError,
            message: err.to_string(),
        })?;

    let mut builder = Client::new()
        .get(url)
        .bearer_auth(access_token)
        .header("Accept", "application/json");

    if let Some(tenant_id) = tenant_id {
        builder = builder.header("X-Smbcloud-Tenant-Id", tenant_id);
    }

    network::request(builder).await
}

fn issuer_base_url(env: Environment) -> Url {
    Url::parse(&format!("{}://{}/", env.api_protocol(), env.api_host()))
        .expect("valid smbcloud api base url")
}
