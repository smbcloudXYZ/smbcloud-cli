use {
    reqwest::Client,
    smbcloud_model::{
        error_codes::ErrorResponse,
        signup::{SignupEmailParams, SignupResult, SignupUserEmail},
    },
    smbcloud_network::{environment::Environment, network::request},
    smbcloud_networking::{constants::PATH_USERS, smb_base_url_builder},
};

pub async fn signup(
    env: Environment,
    user_agent: String,
    email: String,
    password: String,
) -> Result<SignupResult, ErrorResponse> {
    let params = SignupEmailParams {
        user: SignupUserEmail { email, password },
    };
    let builder = Client::new()
        .post(build_smb_signup_url(env))
        .json(&params)
        .header("User-agent", user_agent);
    request(builder).await
}

fn build_smb_signup_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_USERS);
    url_builder.build()
}
