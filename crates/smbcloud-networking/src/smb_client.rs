pub enum SmbClient {
    Cli,
    Sigit,
    WebConsole,
}

impl SmbClient {
    pub fn id(&self) -> &str {
        match self {
            SmbClient::Cli => "cli",
            SmbClient::Sigit => "sigit-app",
            SmbClient::WebConsole => "web",
        }
    }
}
