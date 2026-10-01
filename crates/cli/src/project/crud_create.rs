use crate::ci::{interactive_message, is_ci};
use crate::client;
use crate::token::{get_smb_token::get_smb_token, is_logged_in::is_logged_in};
use crate::{
    account::login::process_login,
    cli::CommandResult,
    project::deploy_target::merge_project_with_frontend_app,
    ui::{fail_message, succeed_message, succeed_symbol},
};
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::console::Term;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use smbcloud_model::frontend_app::{
    AppType, DeployRepoCreate, FrontendApp, FrontendAppCreate, RepoKind,
};
use smbcloud_model::project::{DeploymentMethod, Project, ProjectCreate};
use smbcloud_model::runner::Runner;
use smbcloud_network::environment::Environment;
use smbcloud_networking_project::{
    crud_deploy_repo_create::create_deploy_repo, crud_frontend_app_create::create_frontend_app,
    crud_project_create::create_project,
};
use smbcloud_utils::config::Config as DeployConfig;
use spinners::Spinner;

struct RepoInput {
    repository: String,
    repo_kind: RepoKind,
    runner: Runner,
}

struct AppInput {
    name: String,
    source_path: Option<String>,
    runner: Runner,
}

pub async fn process_project_init(
    env: Environment,
    should_init_project: bool,
) -> Result<CommandResult> {
    // `init` / `project new` are wizards (name, description, repo, runner, …).
    if is_ci() {
        return Err(anyhow!(fail_message(&interactive_message(
            "Project initialization"
        ))));
    }

    let is_logged_in = is_logged_in(env).await?;
    if !is_logged_in {
        let _ = process_login(env, Some(is_logged_in)).await;
    }

    let project_name = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            return Err(anyhow!(fail_message("Invalid project name.")));
        }
    };
    let description = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Description")
        .interact()
    {
        Ok(description) => description,
        Err(_) => {
            return Err(anyhow!(fail_message("Invalid description")));
        }
    };

    // A project is just the workspace. A repo — and the apps that ship from
    // it — can be added now or later.
    let repo_choices = [
        "Single-app repo — one deployable app",
        "Monorepo — several apps in one repo",
        "Skip — add a repo later",
    ];
    let repo_choice = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Add a repo to this project?")
        .items(&repo_choices)
        .default(0)
        .interact_on_opt(&Term::stderr())
    {
        Ok(Some(index)) => index,
        _ => {
            return Err(anyhow!(fail_message("Invalid selection.")));
        }
    };

    let (repo, apps) = match repo_choice {
        0 => {
            let repository = prompt_repository_name(&project_name)?;
            let runner = prompt_runner()?;
            // A single-app repo creates its app automatically server-side.
            (
                Some(RepoInput {
                    repository,
                    repo_kind: RepoKind::SingleApp,
                    runner,
                }),
                Vec::new(),
            )
        }
        1 => {
            let repository = prompt_repository_name(&project_name)?;
            let mut apps = Vec::new();
            loop {
                apps.push(prompt_monorepo_app()?);

                let add_another = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Add another app?")
                    .default(false)
                    .interact()
                    .unwrap_or(false);
                if !add_another {
                    break;
                }
            }
            (
                Some(RepoInput {
                    repository,
                    repo_kind: RepoKind::Monorepo,
                    runner: Runner::Monorepo,
                }),
                apps,
            )
        }
        _ => (None, Vec::new()),
    };

    let spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Creating a project...").green().bold().to_string(),
    );

    let access_token = get_smb_token(env)?;
    let tenant_id = crate::session_config::current_tenant_id(env).unwrap_or(None);
    let project = match create_project(
        env,
        client(),
        access_token.clone(),
        ProjectCreate {
            name: project_name.clone(),
            description: description.clone(),
        },
        tenant_id,
    )
    .await
    {
        Ok(project) => project,
        Err(e) => {
            println!("Error: {e:#?}");
            return Err(anyhow!(fail_message("Failed to create project.")));
        }
    };

    let mut first_app: Option<FrontendApp> = None;
    let mut created_apps = 0;
    if let Some(repo) = &repo {
        let deploy_repo = match create_deploy_repo(
            env,
            client(),
            access_token.clone(),
            DeployRepoCreate {
                project_id: project.id,
                name: repo.repository.clone(),
                repository: repo.repository.clone(),
                repo_kind: repo.repo_kind,
                runner: repo.runner,
                deployment_method: deployment_method_for(repo.runner),
            },
        )
        .await
        {
            Ok(deploy_repo) => deploy_repo,
            Err(e) => {
                println!("Error: {e:#?}");
                return Err(anyhow!(fail_message(
                    "Project created, but failed to create its repo."
                )));
            }
        };

        // A single-app repo ships with its app, created server-side.
        if let Some(embedded_apps) = &deploy_repo.frontend_apps {
            if let Some(embedded_app) = embedded_apps.first() {
                created_apps += 1;
                first_app = Some(embedded_app.clone());
            }
        }

        for app in &apps {
            match create_frontend_app(
                env,
                client(),
                access_token.clone(),
                FrontendAppCreate {
                    name: app.name.clone(),
                    project_id: project.id,
                    app_type: AppType::Web,
                    runner: app.runner,
                    deployment_method: deployment_method_for(app.runner),
                    repository: Some(repo.repository.clone()),
                    description: None,
                    deploy_repo_id: Some(deploy_repo.id),
                    source_path: app.source_path.clone(),
                },
            )
            .await
            {
                Ok(frontend_app) => {
                    created_apps += 1;
                    if first_app.is_none() {
                        first_app = Some(frontend_app);
                    }
                }
                Err(e) => {
                    println!("Error: {e:#?}");
                    return Err(anyhow!(fail_message(&format!(
                        "Project created, but failed to create app {}.",
                        app.name
                    ))));
                }
            }
        }
    }

    if should_init_project {
        write_smb_config(&project, first_app.as_ref())?;
    }

    let msg = match &repo {
        Some(repo) if repo.repo_kind == RepoKind::Monorepo => format!(
            "{project_name} has been created with monorepo {} and {created_apps} app(s).",
            repo.repository
        ),
        Some(repo) => {
            let app_name = first_app
                .as_ref()
                .map(|frontend_app| frontend_app.name.clone())
                .unwrap_or_else(|| repo.repository.clone());
            format!(
                "{project_name} has been created with repo {} and app {app_name}.",
                repo.repository
            )
        }
        None => format!("{project_name} has been created."),
    };

    Ok(CommandResult {
        spinner,
        symbol: succeed_symbol(),
        msg: succeed_message(&msg),
    })
}

fn prompt_repository_name(project_name: &str) -> Result<String> {
    match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Repository name")
        .default(slugify(project_name))
        .interact()
    {
        Ok(repository) => Ok(repository),
        Err(_) => Err(anyhow!(fail_message("Invalid repository name."))),
    }
}

fn prompt_runner() -> Result<Runner> {
    let runners = vec![
        Runner::NodeJs,
        Runner::Static,
        Runner::Ruby,
        Runner::Swift,
        Runner::Rust,
    ];
    match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Runner")
        .items(&runners)
        .default(0)
        .interact_on_opt(&Term::stderr())
    {
        Ok(Some(index)) => Ok(runners[index]),
        _ => Err(anyhow!(fail_message("Invalid runner."))),
    }
}

fn prompt_monorepo_app() -> Result<AppInput> {
    let name = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("App name")
        .interact()
    {
        Ok(name) => name,
        Err(_) => return Err(anyhow!(fail_message("Invalid app name."))),
    };
    let source_path = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Source path (relative to repo root)")
        .default(format!("apps/{}", slugify(&name)))
        .interact()
    {
        Ok(source_path) => source_path,
        Err(_) => return Err(anyhow!(fail_message("Invalid source path."))),
    };
    let runner = prompt_runner()?;

    Ok(AppInput {
        name,
        source_path: Some(source_path),
        runner,
    })
}

// Static sites have no build step to run on the server; they always ship via
// rsync. Everything else defaults to the git flow.
fn deployment_method_for(runner: Runner) -> DeploymentMethod {
    match runner {
        Runner::Static => DeploymentMethod::Rsync,
        _ => DeploymentMethod::Git,
    }
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

fn write_smb_config(workspace_project: &Project, frontend_app: Option<&FrontendApp>) -> Result<()> {
    let deploy_target = match frontend_app {
        Some(frontend_app) => merge_project_with_frontend_app(workspace_project, frontend_app),
        None => workspace_project.clone(),
    };

    let config = DeployConfig {
        name: workspace_project.name.clone(),
        description: workspace_project.description.clone(),
        project: deploy_target,
        projects: None,
    };

    std::fs::create_dir_all(".smb")?;
    std::fs::write(".smb/config.toml", toml::to_string(&config)?)?;
    Ok(())
}
