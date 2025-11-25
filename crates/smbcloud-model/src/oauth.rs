use {
    serde::{Deserialize, Serialize},
    tsync::tsync,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i32,
    pub refresh_token: Option<String>,
    pub refresh_token_expires_in: Option<String>,
    pub scope: String,
    pub token_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct OauthRedirect {
    pub code: String,
    pub scope: String,
    pub authuser: i32,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: String,
}
