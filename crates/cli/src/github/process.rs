use {
    crate::{
        ci,
        cli::CommandResult,
        client,
        github::cli::Commands,
        project::current_project::resolve_required_project_id,
        token::get_smb_token::get_smb_token,
        ui::{prompt, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    smbcloud_model::github::{
        GithubConnection, GithubConnectionCreate, GithubInstallation, GithubRepository,
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking::constants::GH_APP_SLUG,
    smbcloud_networking_project::{
        crud_github_connection::{
            create_github_connection, delete_github_connection, get_github_connection,
            get_github_installation_repositories, get_github_installations,
        },
        crud_project_read::get_project,
    },
    spinners::Spinner,
    std::{collections::HashSet, time::Duration},
};

pub async fn process_github(env: Environment, commands: Commands) -> Result<CommandResult> {
    match commands {
        Commands::Install {} => process_github_install(env).await,
        Commands::Connect {
            repo,
            branch,
            project_id,
        } => process_github_connect(env, repo, branch, project_id).await,
        Commands::Status { project_id } => process_github_status(env, project_id).await,
        Commands::Disconnect { project_id } => process_github_disconnect(env, project_id).await,
    }
}

async fn process_github_install(env: Environment) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    if ci::is_ci() {
        return Err(anyhow!(ci::interactive_message(
            "GitHub App installation (a browser flow)"
        )));
    }

    let existing_ids: HashSet<i64> = get_github_installations(env, client(), access_token.clone())
        .await
        .map_err(api_error)?
        .iter()
        .map(|installation| installation.id)
        .collect();

    let install_url = format!("https://github.com/apps/{GH_APP_SLUG}/installations/new");
    if open::that(&install_url).is_err() {
        println!("Open this URL in your browser to install the app:\n  {install_url}");
    }

    let mut spinner = loading_spinner("Waiting for the installation to complete in your browser");
    let poll_interval = Duration::from_secs(5);
    let poll_attempts = 24;
    for _ in 0..poll_attempts {
        tokio::time::sleep(poll_interval).await;
        let installations = get_github_installations(env, client(), access_token.clone())
            .await
            .map_err(api_error)?;
        if let Some(new_installation) = installations
            .iter()
            .find(|installation| !existing_ids.contains(&installation.id))
        {
            spinner.stop_and_persist(
                &succeed_symbol(),
                succeed_message(&format!(
                    "App installed on {}.",
                    new_installation.account_login
                )),
            );
            return Ok(done_result(
                "Run `smb github connect` to link a repository to your project.",
            ));
        }
    }

    spinner.stop_and_persist(
        &succeed_symbol(),
        succeed_message("No new installation detected yet."),
    );
    Ok(done_result(
        "If you completed the installation in the browser, run `smb github connect` \
         or `smb github status` to continue.",
    ))
}

async fn process_github_connect(
    env: Environment,
    repo: Option<String>,
    branch: Option<String>,
    project_id: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let deploy_repo_id = require_deploy_repo_id(env, access_token.clone(), project_id).await?;

    let mut spinner = loading_spinner("Loading your GitHub App installations");
    let installations = get_github_installations(env, client(), access_token.clone())
        .await
        .map_err(api_error)?;
    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));

    if installations.is_empty() {
        return Err(anyhow!(
            "No smbCloud GitHub App installations found. Run `smb github install` first."
        ));
    }

    let (installation, repository) = match normalize_optional(repo) {
        Some(full_name) => {
            find_repository(env, access_token.clone(), &installations, &full_name).await?
        }
        None => pick_repository(env, access_token.clone(), &installations).await?,
    };

    let payload = GithubConnectionCreate {
        github_installation_id: installation.id,
        github_repo_full_name: repository.full_name.clone(),
        production_branch: normalize_optional(branch),
    };

    let mut spinner = loading_spinner("Connecting the repository");
    let connection = create_github_connection(env, client(), access_token, deploy_repo_id, payload)
        .await
        .map_err(api_error)?;
    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Connected."));
    print_connection(&connection);

    Ok(done_result(&format!(
        "Connected. Pushes to `{}` now deploy automatically.",
        connection.production_branch
    )))
}

async fn process_github_status(
    env: Environment,
    project_id: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let deploy_repo_id = require_deploy_repo_id(env, access_token.clone(), project_id).await?;

    let mut spinner = loading_spinner("Loading the GitHub connection");
    let status = get_github_connection(env, client(), access_token, deploy_repo_id)
        .await
        .map_err(api_error)?;
    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));

    match status.connection.filter(|_| status.connected) {
        Some(connection) => {
            print_connection(&connection);
            Ok(done_result("Done."))
        }
        None => Ok(done_result(
            "Not connected. Run `smb github connect` to set up auto-deploy on push.",
        )),
    }
}

async fn process_github_disconnect(
    env: Environment,
    project_id: Option<String>,
) -> Result<CommandResult> {
    let access_token = get_smb_token(env)?;
    let deploy_repo_id = require_deploy_repo_id(env, access_token.clone(), project_id).await?;

    let mut spinner = loading_spinner("Loading the GitHub connection");
    let status = get_github_connection(env, client(), access_token.clone(), deploy_repo_id)
        .await
        .map_err(api_error)?;
    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));

    let connection = match status.connection.filter(|_| status.connected) {
        Some(connection) => connection,
        None => return Ok(done_result("Nothing to disconnect.")),
    };

    let confirmed = prompt::confirm(
        &format!(
            "Disconnect {} from this project? Pushes will no longer deploy automatically",
            connection.github_repo_full_name
        ),
        false,
    )?;
    if !confirmed {
        return Ok(done_result("Aborted."));
    }

    let mut spinner = loading_spinner("Disconnecting the repository");
    delete_github_connection(env, client(), access_token, deploy_repo_id)
        .await
        .map_err(api_error)?;
    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Disconnected."));

    Ok(done_result("Disconnected."))
}

/// Resolve the project (flag or current project) and return its deploy repo id,
/// the unit a GitHub repository connects to.
async fn require_deploy_repo_id(
    env: Environment,
    access_token: String,
    project_id: Option<String>,
) -> Result<i64> {
    let resolved_project_id = resolve_required_project_id(env, normalize_optional(project_id))?;
    let project = get_project(env, client(), access_token, resolved_project_id)
        .await
        .map_err(api_error)?;
    project.deploy_repo_id.ok_or_else(|| {
        anyhow!("This project has no deploy repo yet. Run `smb deploy` once to initialize it.")
    })
}

/// Locate `full_name` across all installations. Non-interactive, so
/// `--repo owner/name` works in CI.
async fn find_repository(
    env: Environment,
    access_token: String,
    installations: &[GithubInstallation],
    full_name: &str,
) -> Result<(GithubInstallation, GithubRepository)> {
    for installation in installations {
        let repositories = get_github_installation_repositories(
            env,
            client(),
            access_token.clone(),
            installation.id,
        )
        .await
        .map_err(api_error)?;
        if let Some(repository) = repositories
            .into_iter()
            .find(|repository| repository.full_name == full_name)
        {
            return Ok((installation.clone(), repository));
        }
    }
    Err(anyhow!(
        "Repository `{full_name}` is not accessible by any smbCloud GitHub App installation. \
         Check the app's repository access on GitHub, or run `smb github install`."
    ))
}

async fn pick_repository(
    env: Environment,
    access_token: String,
    installations: &[GithubInstallation],
) -> Result<(GithubInstallation, GithubRepository)> {
    let installation = if installations.len() == 1 {
        installations[0].clone()
    } else {
        let index = prompt::select("Select a GitHub account", installations, 0, None)?;
        installations
            .get(index)
            .ok_or_else(|| anyhow!("Invalid selection."))?
            .clone()
    };

    let mut spinner = loading_spinner("Loading repositories");
    let repositories =
        get_github_installation_repositories(env, client(), access_token, installation.id)
            .await
            .map_err(api_error)?;
    spinner.stop_and_persist(&succeed_symbol(), succeed_message("Loaded."));

    if repositories.is_empty() {
        return Err(anyhow!(
            "The installation on {} has no accessible repositories. \
             Grant the app access to a repository on GitHub first.",
            installation.account_login
        ));
    }

    let index = prompt::select("Select a repository", &repositories, 0, None)?;
    let repository = repositories
        .get(index)
        .ok_or_else(|| anyhow!("Invalid selection."))?
        .clone();
    Ok((installation, repository))
}

fn print_connection(connection: &GithubConnection) {
    println!("Repository: {}", connection.github_repo_full_name);
    println!("Production branch: {}", connection.production_branch);
    println!("Installation id: {}", connection.github_installation_id);
    println!("Updated at: {}", connection.updated_at);
}

fn loading_spinner(message: &str) -> Spinner {
    Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        succeed_message(message),
    )
}

fn done_result(message: &str) -> CommandResult {
    CommandResult {
        spinner: loading_spinner("Done"),
        symbol: succeed_symbol(),
        msg: succeed_message(message),
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let normalized_value = value.trim().to_string();
        if normalized_value.is_empty() {
            None
        } else {
            Some(normalized_value)
        }
    })
}

fn api_error(error: impl std::fmt::Display) -> anyhow::Error {
    anyhow!(error.to_string())
}
