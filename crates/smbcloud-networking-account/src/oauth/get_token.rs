use {
    log::debug,
    reqwest::Client,
    serde_json::json,
    smbcloud_model::{
        error_codes::ErrorResponse,
        oauth::{OauthRedirect, TokenResponse},
    },
    smbcloud_network::network::{self},
};

pub async fn get_token(
    oauth_redirect: OauthRedirect,
    client_id: String,
    client_secret: String,
) -> Result<TokenResponse, ErrorResponse> {
    let base_url = format!("https://oauth2.googleapis.com/token");
    debug!("Exchange code with token: {}", oauth_redirect.code);
    let paylod = json!({
        "client_id": client_id,
        "client_secret": client_secret,
        "code": oauth_redirect.code,
        "grant_type": "authorization_code",
        "redirect_uri": "http://localhost:8000"
    });
    let builder = Client::new().post(base_url).json(&paylod);
    network::request(builder).await
}
