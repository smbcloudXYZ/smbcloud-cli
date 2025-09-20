use {
    crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    anyhow::{anyhow, Result},
    console::style,
    git2::{Remote, Repository},
    smbcloud_model::runner::Runner,
    spinners::Spinner,
};

pub async fn remote_deployment_setup<'a>(
    runner: &Runner,
    repo: &'a Repository,
    repo_name: &'a str,
) -> Result<Remote<'a>> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Checking remote deployment...")
            .green()
            .bold()
            .to_string(),
    );

    match repo.find_remote("smbcloud") {
        Ok(remote) => {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("Valid deployment setup."),
            );
            Ok(remote)
        }
        Err(_) => {
            spinner.stop_and_persist(
                &fail_symbol(),
                succeed_message("Remote deployment is not setup. Will setup remote deployment."),
            );
            let mut spinner = Spinner::new(
                spinners::Spinners::Hamburger,
                style("Creating remote deployment...")
                    .green()
                    .bold()
                    .to_string(),
            );
            let repo_name_format = format!("{}.git", repo_name);
            let remote = repo
                .remote(
                    "smbcloud",
                    &format!("{}:{}", runner.git_host(), repo_name_format),
                )
                .map_err(|e: git2::Error| {
                    spinner.stop_and_persist(&fail_symbol(), e.to_string());
                    anyhow!(fail_message("Failed to setup remote deployment: {e}"))
                })?;
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message("Valid deployment setup."),
            );
            Ok(remote)
        }
    }
}
