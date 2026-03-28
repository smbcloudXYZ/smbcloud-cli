use smbcloud_network::environment::Environment;
use smbcloud_networking_account::client_credentials::ClientCredentials;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub async fn logout_with_client(
    env: Environment,
    app_id: String,
    app_secret: String,
    access_token: String,
) -> Result<JsValue, JsValue> {
    let client = ClientCredentials {
        app_id: &app_id,
        app_secret: &app_secret,
    };

    match smbcloud_networking_account::logout::logout_with_client(env, client, access_token).await {
        Ok(_) => Ok(JsValue::UNDEFINED),
        Err(error) => Err(serde_wasm_bindgen::to_value(&error)?),
    }
}
