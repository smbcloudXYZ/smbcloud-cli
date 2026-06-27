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
    /// OIDC identity token, present when the `openid` scope was granted
    /// (smbCloud Auth AuthApp PKCE flow). Absent for plain OAuth responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct OauthRedirect {
    pub code: Option<String>,
    pub scope: Option<String>,
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
