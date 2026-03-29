use {crate::oauth::provider::Provider, uuid::Uuid};

pub fn get_consent_url(provider: Provider) -> String {
    match provider {
        Provider::Google {
            client_id,
            redirect_uri,
        } => format!(
            "https://accounts.google.com/o/oauth2/auth?client_id={}&redirect_uri={}&scope=email%20profile&response_type=code&access_type=offline",
            client_id, redirect_uri
        ),
        Provider::Apple {
            client_id,
            redirect_uri,
            state,
        } => {
            let state_param = state.unwrap_or_else(|| Uuid::new_v4().to_string());

            format!(
                "https://appleid.apple.com/auth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=email%20name&response_mode=form_post&state={}",
                client_id, redirect_uri, state_param
            )
        }
    }
}
