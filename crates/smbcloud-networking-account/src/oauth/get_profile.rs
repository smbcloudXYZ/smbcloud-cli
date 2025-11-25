use {
    log::debug,
    reqwest::Client,
    smbcloud_model::{error_codes::ErrorResponse, oauth::UserInfo},
    smbcloud_network::network::{self},
};

pub async fn get_profile(access_token: String) -> Result<UserInfo, ErrorResponse> {
    let base_url = "https://www.googleapis.com/oauth2/v1/userinfo?alt=json";
    debug!("Get profile with token: {}", access_token);
    let builder = Client::new()
        .get(base_url)
        .header("Authorization", format!("Bearer {}", access_token));
    network::request(builder).await
}
