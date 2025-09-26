use {
    serde::{Deserialize, Serialize},
    wasm_bindgen::prelude::wasm_bindgen,
};

#[derive(clap::ValueEnum, Clone, Copy, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum Environment {
    Dev,
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl std::str::FromStr for Environment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dev" => Ok(Environment::Dev),
            "production" => Ok(Environment::Production),
            _ => Err(()),
        }
    }
}

impl Environment {
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
