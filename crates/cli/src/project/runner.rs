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
    pub fn git_host(&self) -> String {
        format!("git@{}.smbcloud.xyz", self.api())
    }

    fn api(&self) -> &str {
        match self {
            Runner::NodeJs => "api",
            Runner::Ruby | Runner::Swift => "api-1",
        }
    }
}

pub(crate) async fn detect_runner() -> Result<Runner> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Checking runner"),
    );

    if Path::new("package.json").exists()
        && (Path::new("next.config.js").exists()
            || Path::new("next.config.ts").exists()
            || Path::new("next.config.mjs").exists()
            || Path::new("astro.config.mjs").exists())
    {
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
