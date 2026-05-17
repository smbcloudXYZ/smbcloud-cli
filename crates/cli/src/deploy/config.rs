use {
    crate::{
        client,
        deploy::setup_project::setup_project,
        ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    },
    git2::{Cred, CredentialType, Error},
    smbcloud_model::{
        account::User,
        error_codes::{ErrorCode, ErrorResponse},
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking::smb_client::SmbClient,
    smbcloud_networking_project::{
        crud_frontend_app_deploy_config::get_deploy_config, crud_project_read::get_project,
    },
    smbcloud_utils::config::Config,
    spinners::Spinner,
    std::{fs, path::Path},
};

pub(crate) async fn get_config(
    env: Environment,
    access_token: Option<&str>,
) -> Result<Config, ErrorResponse> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        format!(
            "  {} {}",
            console::style("◼").cyan(),
            console::style("Loading config…").dim()
        ),
    );

    // Check .smb directory

    // Get .smb/config.toml file path in the current directory
    let config_path = Path::new(".smb/config.toml");
    if !config_path.exists() {
        spinner.stop_and_persist(&succeed_symbol(), succeed_message("Setting up deployment"));
        // Let's guide the user through the setup process
        setup_project(env, access_token).await?;
        spinner = Spinner::new(
            spinners::Spinners::SimpleDotsScrolling,
            format!(
                "  {} {}",
                console::style("◼").cyan(),
                console::style("Loading config…").dim()
            ),
        );
    }

    // Parse toml file
    let config_content: String =
        fs::read_to_string(config_path).map_err(|_| ErrorResponse::Error {
            error_code: ErrorCode::MissingConfig,
            message: ErrorCode::MissingConfig.message(None).to_string(),
        })?;

    let mut config: Config = match toml::from_str(&config_content) {
        Ok(value) => value,
        Err(e) => {
            println!("Error parsing config: {}", e);
            spinner.stop_and_persist(&fail_symbol(), fail_message("Config unsync."));
            handle_config_error()?
        }
    };
    spinner.stop_and_persist(
        " ",
        format!(
            "  {} {}    {}",
            console::style("◼").cyan(),
            console::style("Config").white().bold(),
            console::style(&config.name).dim(),
        ),
    );

    // Attempt to fetch server-side deploy config when we have both a token
    // and a frontend_app_id. Falls back to local config silently on failure.
    if let Some(token) = access_token {
        if let Some(ref frontend_app_id) = config.project.frontend_app_id {
            match get_deploy_config(env, client(), token.to_string(), frontend_app_id).await {
                Ok(deploy_config) => {
                    let has_server_fields =
                        deploy_config.kind.is_some() || deploy_config.remote_path.is_some();

                    if has_server_fields {
                        println!(
                            "  {}          {}",
                            console::style(" ").dim(),
                            console::style("Loaded from smbcloud.xyz").dim(),
                        );

                        if let Some(remote_kind) = deploy_config.kind {
                            config.project.kind = Some(remote_kind);
                        }
                        if let Some(remote_path) = deploy_config.remote_path {
                            config.project.path = Some(remote_path);
                        }
                        if let Some(remote_output_path) = deploy_config.output_path {
                            config.project.output = Some(remote_output_path);
                        }
                        if let Some(remote_build_command) = deploy_config.build_command {
                            config.project.compile_cmd = Some(remote_build_command);
                        }
                        if let Some(remote_install_command) = deploy_config.install_command {
                            config.project.install_command = Some(remote_install_command);
                        }
                        if let Some(remote_binary_name) = deploy_config.binary_name {
                            config.project.binary_name = Some(remote_binary_name);
                        }
                        if let Some(remote_build_target) = deploy_config.build_target {
                            config.project.rust_target = Some(remote_build_target);
                        }
                        if let Some(remote_package_manager) = deploy_config.package_manager {
                            config.project.package_manager = Some(remote_package_manager);
                        }
                        if let Some(remote_pm2_app) = deploy_config.pm2_app {
                            config.project.pm2_app = Some(remote_pm2_app);
                        }
                        if let Some(remote_port) = deploy_config.port {
                            config.project.port = Some(remote_port);
                        }
                        if let Some(remote_shared_lib_path) = deploy_config.shared_lib_path {
                            config.project.shared_lib = Some(remote_shared_lib_path);
                        }
                        if let Some(remote_source_path) = deploy_config.source_path {
                            config.project.source_path = Some(remote_source_path);
                        }
                        if let Some(remote_repository) = deploy_config.repository {
                            config.project.repository = Some(remote_repository);
                        }
                        config.project.runner = deploy_config.runner;
                        config.project.deployment_method = deploy_config.deployment_method;
                        config.project.deploy_repo_id = deploy_config.deploy_repo_id;
                    } else {
                        println!(
                            "  {}          {}",
                            console::style(" ").dim(),
                            console::style("Loaded from .smb/config.toml").dim(),
                        );
                    }
                }
                Err(_) => {
                    println!(
                        "  {}          {}",
                        console::style(" ").dim(),
                        console::style("Loaded from .smb/config.toml").dim(),
                    );
                }
            }
        }
    }

    Ok(config)
}

fn handle_config_error() -> Result<Config, ErrorResponse> {
    todo!()
}

pub(crate) async fn check_project(
    env: Environment,
    access_token: &str,
    id: i32,
) -> Result<(), ErrorResponse> {
    let mut spinner: Spinner = Spinner::new(
        spinners::Spinners::Hamburger,
        format!(
            "  {} {}",
            console::style("◼").cyan(),
            console::style("Validating project…").dim()
        ),
    );
    match get_project(
        env,
        (&SmbClient::Cli, ""),
        access_token.to_string(),
        id.to_string(),
    )
    .await
    {
        Ok(_) => {
            spinner.stop_and_persist(" ", String::new());
            Ok(())
        }
        Err(_) => {
            spinner.stop_and_persist(&fail_symbol(), succeed_message("Project is unsynched"));
            Err(ErrorResponse::Error {
                error_code: ErrorCode::ProjectNotFound,
                message: ErrorCode::ProjectNotFound.message(None).to_string(),
            })
        }
    }
}

pub fn credentials(
    config: &Config,
    user: User,
) -> impl FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, Error> + '_ {
    move |_url, _username_from_url, _allowed_types| {
        Cred::ssh_key("git", None, Path::new(&config.ssh_key_path(user.id)), None)
    }
}
