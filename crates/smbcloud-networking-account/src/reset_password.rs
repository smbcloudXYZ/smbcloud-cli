use {
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        forgot::{Param, UserUpdatePassword},
    },
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{
        constants::PATH_USERS_PASSWORD, smb_base_url_builder, smb_client::SmbClient,
    },
};

pub async fn reset_password(
    env: Environment,
    client: SmbClient,
    token: String,
    password: String,
) -> Result<(), ErrorResponse> {
    let password_confirmation = password.clone();
    let params = Param {
        user: UserUpdatePassword {
            reset_password_token: token,
            password,
            password_confirmation,
        },
    };

    let builder = Client::new()
        .put(build_smb_reset_password_url(env))
        .json(&params)
        .header("User-agent", client.id())
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");

    request(builder).await
}

fn build_smb_reset_password_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env, &SmbClient::Cli);
    url_builder.add_route(PATH_USERS_PASSWORD);
    url_builder.build()
}
