use {
    crate::{
        deploy::{
            config::Config, setup_create_new_project::create_new_project,
            setup_select_project::select_project,
        },
        token::get_smb_token,
        ui::highlight,
    },
    dialoguer::{theme::ColorfulTheme, Confirm},
    smbcloud_model::{
        error_codes::{ErrorCode, ErrorResponse},
        project::Project,
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::crud_project_read::get_projects,
    std::{env, fs, path::Path},
};

pub(crate) async fn setup_project(env: Environment) -> Result<Config, ErrorResponse> {
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

    let access_token = match get_smb_token(env) {
        Ok(token) => token,
        Err(_) => {
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::Unauthorized,
                message: ErrorCode::Unauthorized.message(None).to_string(),
            })
        }
    };

    let projects = get_projects(env, access_token).await?;

    let project: Project = if !projects.is_empty() {
        select_project(env, projects, &path_str).await?
    } else {
        create_new_project(env, &path_str).await?
    };

    let name = project.name.clone();
    let description = project.description.clone();

    // Create config struct
    let config = Config {
        project,
        name,
        description,
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
