use {
    crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    anyhow::Result,
    smbcloud_model::runner::Runner,
    spinners::Spinner,
    std::{env::current_dir, path::Path},
};

pub(crate) async fn detect_runner() -> Result<Runner> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message("Checking runner"),
    );

    let path = match current_dir() {
        Ok(path) => path,
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
        Runner::NodeJs => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("NodeJs 🟩 runner detected"),
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
