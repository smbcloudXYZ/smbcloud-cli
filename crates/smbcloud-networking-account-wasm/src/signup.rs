use smbcloud_network::environment::Environment;
use smbcloud_networking_account::client_credentials::ClientCredentials;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub async fn signup_with_client(
    env: Environment,
    app_id: String,
    app_secret: String,
    email: String,
    password: String,
) -> Result<JsValue, JsValue> {
    let client = ClientCredentials {
        app_id: &app_id,
        app_secret: &app_secret,
    };

    match smbcloud_networking_account::signup::signup_with_client(env, client, email, password)
        .await
    {
        Ok(response) => Ok(serde_wasm_bindgen::to_value(&response)?),
        Err(error) => Err(serde_wasm_bindgen::to_value(&error)?),
    }
}
