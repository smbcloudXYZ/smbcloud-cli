use {
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        login::{AccountStatus, LoginArgs},
    },
    smbcloud_network::{environment::Environment, network::request_login},
    smbcloud_networking::{
        constants::{PATH_USERS_SIGN_IN, SMB_USER_AGENT},
        smb_base_url_builder,
    },
};

pub async fn login(
    env: Environment,
    username: String,
    password: String,
) -> Result<AccountStatus, ErrorResponse> {
    let login_params = LoginArgs { username, password };
    let builder = Client::new()
        .post(build_smb_login_url(env))
        .json(&login_params)
        .header("User-agent", SMB_USER_AGENT);
    request_login(builder).await
}

pub(crate) fn build_smb_login_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS_SIGN_IN);
    url_builder.build()
}
