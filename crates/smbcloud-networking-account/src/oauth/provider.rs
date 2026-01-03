pub enum Provider {
    Google {
        client_id: String,
        redirect_uri: String,
    },
    Apple {
        client_id: String,
        redirect_uri: String,
        state: Option<String>,
    },
}
