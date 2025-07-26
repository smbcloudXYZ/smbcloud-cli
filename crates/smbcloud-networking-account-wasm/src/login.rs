use crate::url_builder::build_smb_login_url;
use network::environment::Environment;
use network::network::request;
use reqwest::Client;
use serde::Serialize;
use smbcloud_model::login::LoginResult;
use smbcloud_networking::constants::SMB_USER_AGENT;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Serialize)]
pub struct LoginParams {
    pub username: String,
    pub password: String,
}

#[wasm_bindgen]
pub async fn login(
    env: Environment,
    username: String,
    password: String,
) -> Result<JsValue, JsValue> {
    let login_params = LoginParams { username, password };
    let builder = Client::new()
        .post(build_smb_login_url(env))
        .json(&login_params)
        .header("User-agent", SMB_USER_AGENT);
    match request::<LoginResult>(builder).await {
        Ok(response) => Ok(serde_wasm_bindgen::to_value(&response)?),
        Err(error) => Err(serde_wasm_bindgen::to_value(&error)?),
    }
}
