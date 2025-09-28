pub enum SmbClient {
    Cli,
    Sigit,
    WebConsole,
}

impl SmbClient {
    pub fn id(&self) -> &str {
        match self {
            SmbClient::Cli => "CLI_CLIENT_ID",
            SmbClient::Sigit => "SIGIT_CLIENT_ID",
            SmbClient::WebConsole => "WEB_CONSOLE_CLIENT_ID",
        }
    }
    pub fn secret(&self) -> &str {
        match self {
            SmbClient::Cli => "CLI_CLIENT_SECRET",
            SmbClient::Sigit => "SIGIT_CLIENT_SECRET",
            SmbClient::WebConsole => "WEB_CONSOLE_CLIENT_SECRET",
        }
    }
}
