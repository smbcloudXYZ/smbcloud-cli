pub enum SmbClient {
    Cli,
    Sigit,
    WebConsole,
}

impl SmbClient {
    pub fn id(&self) -> &str {
        match self {
            SmbClient::Cli => dotenv!("CLI_CLIENT_ID"),
            SmbClient::Sigit => dotenv!("SIGIT_CLIENT_ID"),
            SmbClient::WebConsole => dotenv!("WEB_CONSOLE_CLIENT_ID"),
        }
    }
    pub fn secret(&self) -> &str {
        match self {
            SmbClient::Cli => dotenv!("CLI_CLIENT_SECRET"),
            SmbClient::Sigit => dotenv!("SIGIT_CLIENT_SECRET"),
            SmbClient::WebConsole => dotenv!("WEB_CONSOLE_CLIENT_SECRET"),
        }
    }
}
