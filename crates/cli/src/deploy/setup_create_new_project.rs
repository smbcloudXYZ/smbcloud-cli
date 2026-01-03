use {
    crate::{client, token::get_smb_token::get_smb_token},
    dialoguer::{console::Term, theme::ColorfulTheme, Input, Select},
    regex::Regex,
    smbcloud_model::{
        error_codes::{ErrorCode, ErrorResponse},
        project::{Project, ProjectCreate},
        runner::Runner,
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::crud_project_create::create_project,
    std::path::Path,
};

pub(crate) async fn create_new_project(
    env: Environment,
    path: &str,
) -> Result<Project, ErrorResponse> {
    let default_name = Path::new(path)
        .file_name()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("project")
        .to_lowercase()
        .replace([' ', '-'], "")
        .replace('-', "");

    let name = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .default(default_name.to_string())
        .interact()
    {
        Ok(project_name) => project_name,
        Err(_) => {
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::InputError,
                message: ErrorCode::InputError.message(None).to_string(),
            });
        }
    };

    // Create a repository name: lowercased, remove spaces and special characters
    let re = Regex::new(r"[^a-zA-Z0-9_-]").unwrap();
    let default_repository = name
        .clone()
        .to_lowercase()
        .replace(' ', "_")
        .replace('-', "");
    let default_repo = re.replace_all(&default_repository, "");

    let repository = match Input::<String>::with_theme(&ColorfulTheme::default())
        .default(default_repo.to_string())
        .with_prompt("Repository")
        .interact()
    {
        Ok(repo) => repo,
        Err(_) => {
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::InputError,
                message: ErrorCode::InputError.message(None).to_string(),
            });
        }
    };

    let description = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Description")
        .interact()
    {
        Ok(description) => description,
        Err(_) => {
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::InputError,
                message: ErrorCode::InputError.message(None).to_string(),
            });
        }
    };

    let runners = vec![Runner::NodeJs, Runner::Swift, Runner::Ruby];
    let runner = Select::with_theme(&ColorfulTheme::default())
        .items(&runners)
        .default(0)
        .interact_on_opt(&Term::stderr())
        .map(|i| runners[i.unwrap()])
        .unwrap();

    let access_token = match get_smb_token(env) {
        Ok(token) => token,
        Err(_) => {
            return Err(ErrorResponse::Error {
                error_code: ErrorCode::Unauthorized,
                message: ErrorCode::Unauthorized.message(None).to_string(),
            })
        }
    };

    match create_project(
        env,
        client(),
        access_token,
        ProjectCreate {
            name,
            runner,
            repository,
            description,
        },
    )
    .await
    {
        Ok(project) => Ok(project),
        Err(e) => Err(e),
    }
}
