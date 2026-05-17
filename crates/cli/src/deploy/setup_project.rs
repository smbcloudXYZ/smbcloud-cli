use {
    crate::{
        client,
        deploy::{
            setup_create_new_project::create_new_project, setup_select_project::select_project,
        },
        project::deploy_target::{
            ensure_default_frontend_app_for_project, merge_project_with_frontend_app,
            resolve_frontend_app_for_project,
        },
        token::get_smb_token::get_smb_token,
        ui::highlight,
    },
    dialoguer::{theme::ColorfulTheme, Confirm},
    smbcloud_model::{
        error_codes::{ErrorCode, ErrorResponse},
        project::Project,
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::crud_project_read::get_projects,
    smbcloud_utils::config::Config,
    std::{env, fs, path::Path},
};

pub(crate) async fn setup_project(
    env: Environment,
    access_token: Option<&str>,
) -> Result<Config, ErrorResponse> {
    let path = env::current_dir().ok();
    let path_str = path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Setup project in {}? y/n", highlight(&path_str)))
        .interact()
        .map_err(|_| ErrorResponse::Error {
            error_code: ErrorCode::InputError,
            message: ErrorCode::InputError.message(None).to_string(),
        })?;

    if !confirm {
        return Err(ErrorResponse::Error {
            error_code: ErrorCode::Cancel,
            message: ErrorCode::Cancel.message(None).to_string(),
        });
    }

    let access_token = match access_token {
        Some(token) => token.to_string(),
        None => match get_smb_token(env) {
            Ok(token) => token,
            Err(_) => {
                return Err(ErrorResponse::Error {
                    error_code: ErrorCode::Unauthorized,
                    message: ErrorCode::Unauthorized.message(None).to_string(),
                })
            }
        },
    };

    let projects = get_projects(env, client(), access_token.to_string()).await?;

    let workspace_project: Project = if !projects.is_empty() {
        select_project(env, projects, &path_str).await?
    } else {
        create_new_project(env, &path_str).await?
    };

    let deploy_target =
        match resolve_frontend_app_for_project(env, &access_token, &workspace_project, true).await?
        {
            Some(frontend_app) => {
                merge_project_with_frontend_app(&workspace_project, &frontend_app)
            }
            None => match ensure_default_frontend_app_for_project(
                env,
                &access_token,
                &workspace_project,
            )
            .await
            {
                Ok(frontend_app) => {
                    merge_project_with_frontend_app(&workspace_project, &frontend_app)
                }
                Err(_) => workspace_project.clone(),
            },
        };

    let name = workspace_project.name.clone();
    let description = workspace_project.description.clone();

    // Create config struct
    let config = Config {
        project: deploy_target,
        name,
        description,
        projects: None,
    };

    // Ensure .smb directory exists
    let smb_dir = Path::new(".smb");
    if !smb_dir.exists() {
        fs::create_dir(smb_dir).map_err(|_| ErrorResponse::Error {
            error_code: ErrorCode::MissingConfig,
            message: ErrorCode::MissingConfig.message(None).to_string(),
        })?;
    }

    // Write config to .smb/config.toml
    let config_toml = toml::to_string(&config).map_err(|_| ErrorResponse::Error {
        error_code: ErrorCode::MissingConfig,
        message: ErrorCode::MissingConfig.message(None).to_string(),
    })?;
    fs::write(".smb/config.toml", config_toml).map_err(|_| ErrorResponse::Error {
        error_code: ErrorCode::MissingConfig,
        message: ErrorCode::MissingConfig.message(None).to_string(),
    })?;

    Ok(config)
}
