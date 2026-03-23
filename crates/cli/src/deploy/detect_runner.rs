use {
    crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    anyhow::Result,
    smbcloud_model::runner::Runner,
    smbcloud_utils::config::Config,
    spinners::Spinner,
    std::{env::current_dir, path::Path},
};

pub(crate) async fn detect_runner(config: &Config) -> Result<Runner> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Checking runner"),
    );

    let path = match current_dir() {
        Ok(path) => path,
        Err(_) => {
            spinner.stop_and_persist(
                &fail_symbol(),
                fail_message("Could not get the current path."),
            );
            anyhow::bail!(
                "Could not detect project runner: no package.json, Gemfile, or Package.swift found"
            );
        }
    };

    // See if we have a monorepo setup.
    if config.project.runner == Runner::Monorepo && config.projects.is_some() {
        return Ok(Runner::Monorepo);
    }
    let runner = match Runner::from(&path) {
        Ok(runner) => runner,
        Err(_) => {
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
    };

    match runner {
        Runner::Monorepo => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("Monorepo universal runner"),
            );
        }
        Runner::NodeJs => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("NodeJs 🟩 runner detected"),
            );
        }
        Runner::Static => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("Static 🌐 site — no build step required"),
            );
        }
        Runner::Ruby => {
            if Path::new("Gemfile").exists() {
                spinner.stop_and_persist(
                    &succeed_symbol(),
                    succeed_message("Ruby 🟥 runner with Rails app detected"),
                );
            }
        }
        Runner::Swift => {
            if Path::new("Package.swift").exists() {
                spinner.stop_and_persist(
                    &succeed_symbol(),
                    succeed_message("Swift 🟧 runner with Vapor app detected"),
                );
            }
        }
    };

    Ok(runner)
}
