use crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol};
use anyhow::{Ok, Result};
use spinners::Spinner;
use std::path::Path;

pub(crate) enum Runner {
    NodeJs,
    Ruby,
}

pub(crate) async fn detect_runner() -> Result<Runner> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Checking runner"),
    );

    if Path::new("package.json").exists() {
        spinner.stop_and_persist(&succeed_symbol(), succeed_message("Detected NodeJs runner"));
        return Ok(Runner::NodeJs);
    }
    if Path::new("Gemfile").exists() {
        spinner.stop_and_persist(&succeed_symbol(), succeed_message("Detected Ruby runner"));
        return Ok(Runner::Ruby);
    }

    spinner.stop_and_persist(
        &fail_symbol(),
        fail_message("Could not detect project runner: no package.json or Gemfile found"),
    );
    anyhow::bail!("Could not detect project runner: no package.json or Gemfile found");
}
