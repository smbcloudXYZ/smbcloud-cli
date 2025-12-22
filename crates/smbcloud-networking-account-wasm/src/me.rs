use smbcloud_network::environment::Environment;
use smbcloud_networking::smb_client::SmbClient;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub async fn me(env: Environment, access_token: String) -> Result<JsValue, JsValue> {
    match smbcloud_networking_account::me::me(env, SmbClient::Cli, &access_token).await {
        Ok(response) => Ok(serde_wasm_bindgen::to_value(&response)?),
        Err(error) => Err(serde_wasm_bindgen::to_value(&error)?),
    }
}
