use crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol};
use anyhow::{Ok, Result};
use spinners::Spinner;
use std::path::Path;

pub(crate) enum Runner {
    NodeJs,
    Ruby,
    Swift,
}

impl Runner {
    pub fn git_host(&self) -> &str {
        match self {
            Runner::Swift => "git@api.musik88.com",
            _ => "git@api.smbcloud.xyz",
        }
    }
}

pub(crate) async fn detect_runner() -> Result<Runner> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Checking runner"),
    );

    if Path::new("package.json").exists() {
        spinner.stop_and_persist(
            &succeed_symbol(),
            succeed_message("NodeJs 🟩 runner detected"),
        );
        return Ok(Runner::NodeJs);
    }
    if Path::new("Gemfile").exists() {
        spinner.stop_and_persist(
            &succeed_symbol(),
            succeed_message("Ruby 🟥 runner detected"),
        );
        return Ok(Runner::Ruby);
    }
    if Path::new("Package.swift").exists() {
        spinner.stop_and_persist(
            &succeed_symbol(),
            succeed_message("Swift 🟧 runner detected"),
        );
        return Ok(Runner::Swift);
    }

    spinner.stop_and_persist(
        &fail_symbol(),
        fail_message(
            "Could not detect project runner: no package.json, Gemfile, or Package.swift found",
        ),
    );
    anyhow::bail!(
        "Could not detect project runner: no package.json, Gemfile, or Package.swift found"
    );
}
