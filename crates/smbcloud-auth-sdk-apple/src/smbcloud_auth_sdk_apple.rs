uniffi::setup_scaffolding!();

use smbcloud_auth_sdk::{
    apple, client_credentials::ClientCredentials, login, logout, me, oidc, remove, signup,
};
use smbcloud_model::error_codes::ErrorResponse;
use smbcloud_network::environment::Environment;

// ── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SmbCloudAuthError {
    #[error("[{error_code}] {message}")]
    Api { error_code: i32, message: String },
}

impl From<ErrorResponse> for SmbCloudAuthError {
    fn from(error: ErrorResponse) -> Self {
        match error {
            ErrorResponse::Error {
                error_code,
                message,
            } => Self::Api {
                error_code: error_code as i32,
                message,
            },
        }
    }
}

// ── Environment ──────────────────────────────────────────────────────────────

#[derive(uniffi::Enum)]
pub enum SmbCloudEnvironment {
    Dev,
    Production,
}

impl From<SmbCloudEnvironment> for Environment {
    fn from(environment: SmbCloudEnvironment) -> Self {
        match environment {
            SmbCloudEnvironment::Dev => Self::Dev,
            SmbCloudEnvironment::Production => Self::Production,
        }
    }
}

// ── Account status ───────────────────────────────────────────────────────────

#[derive(uniffi::Enum)]
pub enum SmbCloudAccountStatus {
    NotFound,
    Ready { access_token: String },
    Incomplete { error_code: u32 },
}

impl From<smbcloud_model::login::AccountStatus> for SmbCloudAccountStatus {
    fn from(status: smbcloud_model::login::AccountStatus) -> Self {
        match status {
            smbcloud_model::login::AccountStatus::NotFound => Self::NotFound,
            smbcloud_model::login::AccountStatus::Ready { access_token } => {
                Self::Ready { access_token }
            }
            smbcloud_model::login::AccountStatus::Incomplete { status } => Self::Incomplete {
                error_code: status as u32,
            },
        }
    }
}

// ── User ─────────────────────────────────────────────────────────────────────

#[derive(uniffi::Record)]
pub struct SmbCloudUser {
    pub id: i32,
    pub email: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<smbcloud_model::account::User> for SmbCloudUser {
    fn from(user: smbcloud_model::account::User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
        }
    }
}

// ── Signup result ────────────────────────────────────────────────────────────

#[derive(uniffi::Record)]
pub struct SmbCloudSignupResult {
    pub code: Option<i32>,
    pub message: String,
    pub user_id: Option<i32>,
    pub user_email: Option<String>,
    pub user_created_at: Option<String>,
}

impl From<smbcloud_model::signup::SignupResult> for SmbCloudSignupResult {
    fn from(result: smbcloud_model::signup::SignupResult) -> Self {
        Self {
            code: result.code,
            message: result.message,
            user_id: result.data.as_ref().map(|data| data.id),
            user_email: result.data.as_ref().map(|data| data.email.clone()),
            user_created_at: result.data.map(|data| data.created_at),
        }
    }
}

// ── OIDC types ───────────────────────────────────────────────────────────────

#[derive(uniffi::Record)]
pub struct SmbCloudAuthorizationRequest {
    pub authorize_url: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_verifier: String,
}

impl From<oidc::AuthorizationRequest> for SmbCloudAuthorizationRequest {
    fn from(request: oidc::AuthorizationRequest) -> Self {
        Self {
            authorize_url: request.authorize_url,
            redirect_uri: request.redirect_uri,
            state: request.state,
            code_verifier: request.code_verifier,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SmbCloudCallbackPayload {
    pub code: String,
    pub state: String,
}

impl From<oidc::CallbackPayload> for SmbCloudCallbackPayload {
    fn from(payload: oidc::CallbackPayload) -> Self {
        Self {
            code: payload.code,
            state: payload.state,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SmbCloudTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i32>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

impl From<oidc::TokenResponse> for SmbCloudTokenResponse {
    fn from(response: oidc::TokenResponse) -> Self {
        Self {
            access_token: response.access_token,
            token_type: response.token_type,
            expires_in: response.expires_in,
            refresh_token: response.refresh_token,
            scope: response.scope,
            id_token: response.id_token,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SmbCloudUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub tenant_id: Option<u64>,
    pub tenant_slug: Option<String>,
}

impl From<oidc::UserInfo> for SmbCloudUserInfo {
    fn from(info: oidc::UserInfo) -> Self {
        Self {
            sub: info.sub,
            email: info.email,
            email_verified: info.email_verified,
            tenant_id: info.tenant_id,
            tenant_slug: info.tenant_slug,
        }
    }
}

// ── Apple Sign-In types ──────────────────────────────────────────────────────

#[derive(uniffi::Record)]
pub struct SmbCloudAppleAuthorizationRequest {
    pub authorize_url: String,
    pub redirect_uri: String,
    pub state: String,
}

impl From<apple::AppleAuthorizationRequest> for SmbCloudAppleAuthorizationRequest {
    fn from(request: apple::AppleAuthorizationRequest) -> Self {
        Self {
            authorize_url: request.authorize_url,
            redirect_uri: request.redirect_uri,
            state: request.state,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SmbCloudAppleAuthSession {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub provider: String,
    pub provider_account_id: String,
    pub state: Option<String>,
}

impl From<apple::AppleAuthSession> for SmbCloudAppleAuthSession {
    fn from(session: apple::AppleAuthSession) -> Self {
        Self {
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            email: session.email,
            name: session.name,
            provider: session.provider,
            provider_account_id: session.provider_account_id,
            state: session.state,
        }
    }
}

// ── Main client object ──────────────────────────────────────────────────────

#[derive(uniffi::Object)]
pub struct SmbCloudAuth {
    environment: Environment,
    app_id: String,
    app_secret: String,
}

impl SmbCloudAuth {
    fn credentials(&self) -> ClientCredentials<'_> {
        ClientCredentials {
            app_id: &self.app_id,
            app_secret: &self.app_secret,
        }
    }
}

#[uniffi::export]
impl SmbCloudAuth {
    #[uniffi::constructor]
    pub fn new(environment: SmbCloudEnvironment, app_id: String, app_secret: String) -> Self {
        Self {
            environment: environment.into(),
            app_id,
            app_secret,
        }
    }

    // ── Email/password auth ──────────────────────────────────────────────

    pub async fn login(
        &self,
        email: String,
        password: String,
    ) -> Result<SmbCloudAccountStatus, SmbCloudAuthError> {
        login::login_with_client(self.environment, self.credentials(), email, password)
            .await
            .map(Into::into)
            .map_err(Into::into)
    }

    pub async fn signup(
        &self,
        email: String,
        password: String,
    ) -> Result<SmbCloudSignupResult, SmbCloudAuthError> {
        signup::signup_with_client(self.environment, self.credentials(), email, password)
            .await
            .map(Into::into)
            .map_err(Into::into)
    }

    pub async fn logout(&self, access_token: String) -> Result<(), SmbCloudAuthError> {
        logout::logout_with_client(self.environment, self.credentials(), access_token)
            .await
            .map_err(Into::into)
    }

    pub async fn me(&self, access_token: String) -> Result<SmbCloudUser, SmbCloudAuthError> {
        me::me_with_client(self.environment, self.credentials(), &access_token)
            .await
            .map(Into::into)
            .map_err(Into::into)
    }

    pub async fn remove_account(&self, access_token: String) -> Result<(), SmbCloudAuthError> {
        remove::remove_with_client(self.environment, self.credentials(), &access_token)
            .await
            .map_err(Into::into)
    }

    // ── Apple Sign-In ────────────────────────────────────────────────────

    pub fn build_apple_authorization_request(
        &self,
        redirect_uri: String,
        state: Option<String>,
    ) -> Result<SmbCloudAppleAuthorizationRequest, SmbCloudAuthError> {
        apple::build_authorization_request_with_client(
            self.environment,
            self.credentials(),
            redirect_uri,
            state,
        )
        .map(Into::into)
        .map_err(Into::into)
    }

    pub fn parse_apple_callback_url(
        &self,
        callback_url: String,
        expected_state: Option<String>,
    ) -> Result<SmbCloudAppleAuthSession, SmbCloudAuthError> {
        apple::parse_callback_url(&callback_url, expected_state.as_deref())
            .map(Into::into)
            .map_err(Into::into)
    }

    // ── OIDC ─────────────────────────────────────────────────────────────

    pub fn build_oidc_authorization_request(
        &self,
        oidc_client_id: String,
        redirect_uri: String,
    ) -> Result<SmbCloudAuthorizationRequest, SmbCloudAuthError> {
        oidc::build_authorization_request(self.environment, &oidc_client_id, redirect_uri)
            .map(Into::into)
            .map_err(Into::into)
    }

    pub fn parse_oidc_callback_url(
        &self,
        callback_url: String,
    ) -> Result<SmbCloudCallbackPayload, SmbCloudAuthError> {
        oidc::parse_callback_url(&callback_url)
            .map(Into::into)
            .map_err(Into::into)
    }

    pub async fn exchange_oidc_code(
        &self,
        oidc_client_id: String,
        redirect_uri: String,
        code: String,
        code_verifier: String,
    ) -> Result<SmbCloudTokenResponse, SmbCloudAuthError> {
        oidc::exchange_code(
            self.environment,
            &oidc_client_id,
            &redirect_uri,
            &code,
            &code_verifier,
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
    }

    pub async fn get_oidc_userinfo(
        &self,
        access_token: String,
        tenant_id: Option<String>,
    ) -> Result<SmbCloudUserInfo, SmbCloudAuthError> {
        oidc::get_userinfo(self.environment, &access_token, tenant_id.as_deref())
            .await
            .map(Into::into)
            .map_err(Into::into)
    }
}
