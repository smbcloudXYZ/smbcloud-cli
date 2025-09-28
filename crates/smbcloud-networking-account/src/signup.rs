use {
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        signup::{SignupEmailParams, SignupResult, SignupUserEmail},
    },
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::PATH_USERS, smb_base_url_builder, smb_client::SmbClient},
};

pub async fn signup(
    env: Environment,
    client: SmbClient,
    email: String,
    password: String,
) -> Result<SignupResult, ErrorResponse> {
    let params = SignupEmailParams {
        user: SignupUserEmail { email, password },
    };
    let builder = Client::new()
        .post(build_smb_signup_url(env, &client))
        .json(&params)
        .header("User-agent", client.id());
    request(builder).await
}

fn build_smb_signup_url(env: Environment, client: &SmbClient) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route(PATH_USERS);
    url_builder.build()
}
