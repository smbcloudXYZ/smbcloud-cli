pub enum SmbClient {
    Cli,
    Sigit,
    Moovibe,
    WebConsole,
}

impl SmbClient {
    pub fn id(&self) -> &str {
        match self {
            Self::Cli => "cli",
            Self::Sigit => "sigit-app",
            Self::Moovibe => "moovibe-app",
            Self::WebConsole => "web",
        }
    }
}
