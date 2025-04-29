use crate::cli::CommandResult;
use anyhow::Result;
use console::style;
use smbcloud_model::project::ProjectCreate;
use smbcloud_networking_project::create_project;
use spinners::Spinner;

pub async fn process_project_init(
    name: Option<String>,
    description: Option<String>,
) -> Result<CommandResult> {
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Loading...").green().bold().to_string(),
    );
    let name = name.unwrap_or_else(|| "New Project".to_string());
    let description = description.unwrap_or_else(|| "No description".to_string());
    let project = ProjectCreate { name, description };
    match create_project(project).await {
        Ok(p) => {
            spinner.stop_and_persist("✅", "Done.".to_owned());
            Ok(CommandResult {
                spinner,
                symbol: "✅".to_owned(),
                msg: format!("Project {} has been created.", p.name),
            })
        }
        Err(e) => {
            println!("Error: {e:#?}");
            Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: format!("Failed to initiate a project."),
            })
        }
    }
}
