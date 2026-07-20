use crate::tenant::{
    request::request_empty,
    url_builder::{build_tenant_url, build_tenants_url},
};
use reqwest::Client;
use serde::Serialize;
use smbcloud_model::{
    error_codes::ErrorResponse,
    tenant::{Tenant, TenantCreate, TenantUpdate},
};
use smbcloud_network::{environment::Environment, network::request};
use smbcloud_networking::{constants::SMB_USER_AGENT, smb_client::SmbClient};

#[derive(Serialize)]
struct TenantEnvelope<T> {
    tenant: T,
}

pub async fn get_tenants(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
) -> Result<Vec<Tenant>, ErrorResponse> {
    let builder = Client::new()
        .get(build_tenants_url(env, client))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn get_tenant(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    tenant_id: String,
) -> Result<Tenant, ErrorResponse> {
    let builder = Client::new()
        .get(build_tenant_url(env, client, &tenant_id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn create_tenant(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    tenant: TenantCreate,
) -> Result<Tenant, ErrorResponse> {
    let builder = Client::new()
        .post(build_tenants_url(env, client))
        .json(&TenantEnvelope { tenant })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn update_tenant(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    tenant_id: String,
    tenant: TenantUpdate,
) -> Result<Tenant, ErrorResponse> {
    let builder = Client::new()
        .patch(build_tenant_url(env, client, &tenant_id))
        .json(&TenantEnvelope { tenant })
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request(builder).await
}

pub async fn delete_tenant(
    env: Environment,
    client: (&SmbClient, &str),
    access_token: String,
    tenant_id: String,
) -> Result<(), ErrorResponse> {
    let builder = Client::new()
        .delete(build_tenant_url(env, client, &tenant_id))
        .header("Authorization", access_token)
        .header("User-agent", SMB_USER_AGENT);
    request_empty(builder).await
}
