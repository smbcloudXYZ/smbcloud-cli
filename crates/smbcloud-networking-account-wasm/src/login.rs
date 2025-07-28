use smbcloud_network::environment::Environment;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub async fn login(
    env: Environment,
    username: String,
    password: String,
) -> Result<JsValue, JsValue> {
    match smbcloud_networking_account::login::login(env, username, password).await {
        Ok(response) => Ok(serde_wasm_bindgen::to_value(&response)?),
        Err(error) => Err(serde_wasm_bindgen::to_value(&error)?),
    }
}
