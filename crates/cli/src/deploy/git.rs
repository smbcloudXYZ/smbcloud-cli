use crate::ui::{fail_message, fail_symbol, succeed_message, succeed_symbol};
use anyhow::{anyhow, Result};
use console::style;
use git2::{Remote, Repository};
use spinners::Spinner;

pub async fn remote_deployment_setup(repo: &Repository, repo_name: String) -> Result<Remote> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Setting up remote deployment...")
            .green()
            .bold()
            .to_string(),
    );

    let smbcloud = match repo.find_remote("smbcloud") {
        Ok(remote) => remote,
        Err(_) => {
            spinner.stop_and_persist(
                &fail_symbol(),
                fail_message("Remote deployment is not setup. Will setup remote deployment."),
            );
            // Present the user with a message to setup remote deployment
            repo.remote(
                "smbcloud",
                &format!("deploy@api.smbcloud.xyz:git/{}", repo_name),
            )
            .map_err(|e: git2::Error| {
                spinner.stop_and_persist(&fail_symbol(), e.to_string());
                anyhow!(fail_message("Failed to setup remote deployment: {e}"))
            })?;
            return Err(anyhow!(fail_message(
                "Remote deployment is not setup. Please run `git remote add smbcloud <url>`."
            )));
        }
    };

    spinner.stop_and_persist(
        &succeed_symbol(),
        succeed_message("Deployment setup complete."),
    );

    Ok(smbcloud)
}
