#[derive(clap::ValueEnum, Clone, Copy)]
pub enum Environment {
    Dev,
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl Environment {
    pub fn from_str(env: &str) -> Self {
        match env.to_lowercase().as_str() {
            "dev" => Environment::Dev,
            "production" => Environment::Production,
            _ => panic!("Invalid environment: {}", env),
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            Environment::Dev => "dev",
            Environment::Production => "production",
        }
    }

    pub fn smb_dir(&self) -> String {
        match self {
            Environment::Dev => ".smb-dev".to_string(),
            Environment::Production => ".smb".to_string(),
        }
    }

    pub fn api_protocol(&self) -> String {
        match self {
            Environment::Dev => "http".to_string(),
            Environment::Production => "https".to_string(),
        }
    }
    pub fn api_host(&self) -> String {
        match self {
            Environment::Dev => "localhost:8088".to_string(),
            Environment::Production => "api.smbcloud.xyz".to_string(),
        }
    }
}
