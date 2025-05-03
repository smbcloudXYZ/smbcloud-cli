use anyhow::{Result, anyhow};
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
            spinner.stop_and_persist("😩", "Remote deployment is not setup. Will setup remote deployment.".to_owned());
            // Present the user with a message to setup remote deployment
            repo.remote("smbcloud", &format!("deploy@api.smbcloud.xyz:git/{}", repo_name))
                .map_err(|e: git2::Error| {
                    spinner.stop_and_persist("😩", e.to_string());
                    anyhow!("Failed to setup remote deployment: {e}")
                })?;
            return Err(anyhow!(
                "Remote deployment is not setup. Please run `git remote add smbcloud <url>`."
            ));
        }
    };  

    spinner.stop_and_persist("🚀", "Remote deployment setup complete.".to_owned());

    Ok(smbcloud)
}
