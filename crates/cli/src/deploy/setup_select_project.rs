use {
    crate::deploy::setup_create_new_project::create_new_project,
    dialoguer::{theme::ColorfulTheme, Confirm, Select},
    smbcloud_model::{
        error_codes::{ErrorCode, ErrorResponse},
        project::Project,
    },
    smbcloud_network::environment::Environment,
};

pub(crate) async fn select_project(
    env: Environment,
    projects: Vec<Project>,
    path: &str,
) -> Result<Project, ErrorResponse> {
    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Use existing project? y/n")
        .interact()
        .map_err(|_| ErrorResponse::Error {
            error_code: ErrorCode::InputError,
            message: ErrorCode::InputError.message(None).to_string(),
        })?;

    if !confirm {
        return create_new_project(env, path).await;
    }
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&projects)
        .default(0)
        .interact()
        .map_err(|_| ErrorResponse::Error {
            error_code: ErrorCode::InputError,
            message: ErrorCode::InputError.message(None).to_string(),
        })?;

    let project = projects[selection].clone();

    Ok(project)
}
