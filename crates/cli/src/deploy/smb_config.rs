use crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol};
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

    // Check .smb directory

    // Get .smb/config.toml file path in the current directory
    let config_path = Path::new(".smb/config.toml");
    if !config_path.exists() {
        spinner.stop_and_persist(&fail_symbol(), fail_message("Invalid config."));
        return Err(anyhow::anyhow!(fail_message(
            "No config file found. Please run `smbcloud init` command."
        )));
    }

    // Parse toml file
    let config_content = fs::read_to_string(config_path)?;
    let config: Value = toml::from_str(&config_content)?;

    let repo_name = config
        .get("repository")
        .and_then(|repo| repo.get("name"))
        .and_then(|name| name.as_str())
        .ok_or_else(|| anyhow::anyhow!(fail_message("Repo name not found in config file.")))?;

    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Valid config."));

    Ok(repo_name.to_string())
}
