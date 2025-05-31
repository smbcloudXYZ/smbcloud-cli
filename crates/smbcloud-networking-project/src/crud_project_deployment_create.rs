use crate::build_project_deployment_index;
use anyhow::{anyhow, Result};
use log::debug;
use reqwest::Client;
use smbcloud_model::project::{Deployment, DeploymentPayload};
use smbcloud_networking::{constants::SMB_USER_AGENT, environment::Environment, get_smb_token};

pub async fn create(
    env: Environment,
    project_id: i32,
    payload: DeploymentPayload,
) -> Result<Deployment> {
    // Get current token
    let token = get_smb_token(env).await?;

    debug!("Current token: {}", token);

    let response = Client::new()
        .post(build_project_deployment_index(env, project_id.to_string()))
        .json(&payload)
        .header("Authorization", token)
        .header("User-agent", SMB_USER_AGENT)
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::CREATED => {
            let deployment: Deployment = response.json().await?;
            Ok(deployment)
            // After receiving the response
            //let body = response.text().await?;
            //println!("Response body: {}", body);
            // todo!()
        }
        _ => Err(anyhow!("Something wrong.")),
    }
}