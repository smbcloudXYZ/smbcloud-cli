use smbcloud_auth::client_credentials::ClientCredentials;
use smbcloud_network::environment::Environment;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn build_apple_authorization_request_with_client(
    env: Environment,
    app_id: String,
    app_secret: String,
    redirect_uri: String,
    state: Option<String>,
) -> Result<JsValue, JsValue> {
    let client = ClientCredentials {
        app_id: &app_id,
        app_secret: &app_secret,
    };

    match smbcloud_auth::apple::build_authorization_request_with_client(
        env,
        client,
        redirect_uri,
        state,
    ) {
        Ok(response) => Ok(serde_wasm_bindgen::to_value(&response)?),
        Err(error) => Err(serde_wasm_bindgen::to_value(&error)?),
    }
}

#[wasm_bindgen]
pub fn parse_apple_callback_url(
    callback_url: String,
    expected_state: Option<String>,
) -> Result<JsValue, JsValue> {
    match smbcloud_auth::apple::parse_callback_url(
        &callback_url,
        expected_state.as_deref(),
    ) {
        Ok(response) => Ok(serde_wasm_bindgen::to_value(&response)?),
        Err(error) => Err(serde_wasm_bindgen::to_value(&error)?),
    }
}
