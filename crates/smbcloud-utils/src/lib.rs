use {
    anyhow::{anyhow, Result},
    log::debug,
    regex::Regex,
    smbcloud_model::project::Config,
    std::{fs::OpenOptions, io::Write},
};

pub mod config;

pub fn email_validation(input: &str) -> Result<(), &'static str> {
    let email_regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )
    .unwrap();

    if email_regex.is_match(input) {
        Ok(())
    } else {
        Err("Username must be an email address.")
    }
}

pub async fn get_config() -> Result<Config> {
    match home::home_dir() {
        Some(mut path) => {
            path.push(".smb/config.json");
            if !path.exists() {
                let config = Config {
                    current_project: None,
                    current_auth_app: None,
                };
                return Ok(config);
            }
            let config_string = std::fs::read_to_string(path).map_err(|e| {
                debug!("Error while reading config file: {}", &e);
                anyhow!("Error while reading config file. Are you logged in?")
            })?;
            let config: Config = serde_json::from_str(&config_string).map_err(|e| {
                debug!("Error while parsing config: {}", &e);
                anyhow!("Error while parsing config. Are you logged in?")
            })?;

            Ok(config)
        }
        None => {
            let config = Config {
                current_project: None,
                current_auth_app: None,
            };
            Ok(config)
        }
    }
}

pub fn write_config(config: Config) -> Result<Config> {
    match home::home_dir() {
        Some(path) => {
            debug!("{}", path.to_str().unwrap());
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open([path.to_str().unwrap(), "/.smb/config.json"].join(""))?;
            let json = serde_json::to_string(&config)?;
            file.write_all(json.as_bytes())?;

            Ok(config)
        }
        None => Err(anyhow!("Error getting home directory.")),
    }
}
