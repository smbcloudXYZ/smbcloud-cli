pub fn get_consent_url(client_id: String, redirect_uri: String) -> String {
    format!(
        "https://accounts.google.com/o/oauth2/auth?client_id={}&redirect_uri={}&scope=email%20profile&response_type=code&access_type=offline",
        client_id, redirect_uri
    )
}
