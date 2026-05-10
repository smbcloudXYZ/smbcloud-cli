use {
    crate::client,
    dialoguer::{console::Term, theme::ColorfulTheme, Select},
    smbcloud_model::{
        frontend_app::{AppType, FrontendApp, FrontendAppCreate},
        project::Project,
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::{
        crud_frontend_app_create::create_frontend_app,
        crud_frontend_app_read::get_frontend_apps_by_project,
    },
};

pub async fn resolve_frontend_app_for_project(
    env: Environment,
    access_token: &str,
    project: &Project,
    interactive: bool,
) -> Result<Option<FrontendApp>, smbcloud_model::error_codes::ErrorResponse> {
    let frontend_apps =
        get_frontend_apps_by_project(env, client(), access_token.to_string(), project.id).await?;

    if frontend_apps.is_empty() {
        return Ok(None);
    }

    if frontend_apps.len() == 1 || !interactive {
        return Ok(frontend_apps.into_iter().next());
    }

    let labels: Vec<String> = frontend_apps
        .iter()
        .map(|frontend_app| {
            let source_path = frontend_app.source_path.as_deref().unwrap_or(".");
            let repo_name = frontend_app
                .deploy_repo
                .as_ref()
                .map(|deploy_repo| deploy_repo.repository.as_str())
                .or(frontend_app.repository.as_deref())
                .unwrap_or("no-repo");

            format!("{}  [{}]  {}", frontend_app.name, repo_name, source_path)
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select app target")
        .items(&labels)
        .default(0)
        .interact_on_opt(&Term::stderr())
        .ok()
        .flatten();

    Ok(selection.map(|index| frontend_apps[index].clone()))
}

pub async fn ensure_default_frontend_app_for_project(
    env: Environment,
    access_token: &str,
    project: &Project,
) -> Result<FrontendApp, smbcloud_model::error_codes::ErrorResponse> {
    create_frontend_app(
        env,
        client(),
        access_token.to_string(),
        FrontendAppCreate {
            name: project.name.clone(),
            project_id: project.id,
            app_type: AppType::Web,
            runner: project.runner,
            deployment_method: project.deployment_method,
            repository: project.repository.clone(),
            description: project.description.clone(),
        },
    )
    .await
}

pub fn merge_project_with_frontend_app(project: &Project, frontend_app: &FrontendApp) -> Project {
    let mut deploy_target = project.clone();
    deploy_target.name = frontend_app.name.clone();
    deploy_target.description = frontend_app
        .description
        .clone()
        .or_else(|| project.description.clone());
    deploy_target.runner = frontend_app.runner;
    deploy_target.deployment_method = frontend_app.deployment_method;
    deploy_target.repository = frontend_app
        .deploy_repo
        .as_ref()
        .map(|deploy_repo| deploy_repo.repository.clone())
        .or_else(|| frontend_app.repository.clone())
        .or_else(|| project.repository.clone());
    deploy_target.frontend_app_id = Some(frontend_app.id.clone());
    deploy_target.deploy_repo_id = frontend_app.deploy_repo_id;
    deploy_target.source_path = frontend_app.source_path.clone();

    if deploy_target.source.is_none() {
        deploy_target.source = frontend_app.source_path.clone();
    }

    deploy_target
}
