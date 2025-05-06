use anyhow::Result;
use console::style;
use spinners::Spinner;
use std::{fs, path::Path};
use toml::Value;

pub(crate) async fn check_config() -> Result<String> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Checking config...").green().bold().to_string(),
    );

    // Get .smb/config.toml file path in the current directory
    let config_path = Path::new(".smb/config.toml");
    if !config_path.exists() {
        spinner.stop_and_persist("😩", "No config file found.".to_owned());
        return Err(anyhow::anyhow!(
            "No config file found. Please run `smbcloud init` command."
        ));
    }

    // Parse toml file
    let config_content = fs::read_to_string(config_path)?;
    let config: Value = toml::from_str(&config_content)?;

    let repo_name = config
        .get("repo")
        .and_then(|repo| repo.get("name"))
        .and_then(|name| name.as_str())
        .ok_or_else(|| anyhow::anyhow!("Repo name not found in config file"))?;

    Ok(repo_name.to_string())
}
