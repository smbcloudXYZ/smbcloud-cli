//! Plain-text renderers for the headless interface.
//!
//! The default (headless) interface prints line-based plain text instead of
//! taking over the terminal with a full-screen `ratatui` view. These renderers
//! are the headless counterparts to the `show_*_tui` entry points; a handler
//! chooses between them with [`crate::interface::is_tui`].

use {
    crate::ui::highlight,
    smbcloud_model::{
        account::User,
        project::{Deployment, Project},
    },
    tabled::{builder::Builder, settings::Style},
};

/// Render the authenticated user's account details as a plain key/value block.
pub fn render_user(user: &User) {
    println!();
    println!("  {}", highlight(&user.email));
    println!("  {:<14}{}", "ID", user.id);
    println!(
        "  {:<14}{}",
        "Member since",
        user.created_at.format("%Y-%m-%d")
    );
    println!(
        "  {:<14}{}",
        "Last updated",
        user.updated_at.format("%Y-%m-%d")
    );
    println!();
}

/// Render the project list as a plain table.
pub fn render_projects(projects: &[Project]) {
    if projects.is_empty() {
        println!("No projects found.");
        return;
    }
    let mut builder = Builder::default();
    builder.push_record(["ID", "Name", "Runner", "Repository"]);
    for project in projects {
        builder.push_record([
            project.id.to_string(),
            project.name.clone(),
            project.runner.to_string(),
            project.repository.clone().unwrap_or_default(),
        ]);
    }
    let mut table = builder.build();
    table.with(Style::rounded());
    println!("{table}");
}

/// Render a single project's details as a plain key/value block.
pub fn render_project_detail(project: &Project) {
    println!();
    println!("  {}", highlight(&project.name));
    println!("  {:<16}{}", "ID", project.id);
    println!("  {:<16}{}", "Runner", project.runner);
    println!("  {:<16}{}", "Deployment", project.deployment_method);
    field_opt("Repository", project.repository.as_deref());
    field_opt("Description", project.description.as_deref());
    field_opt("Path", project.path.as_deref());
    println!();
}

/// Render the deployment list as a plain table.
pub fn render_deployments(deployments: &[Deployment]) {
    if deployments.is_empty() {
        println!("No deployments found.");
        return;
    }
    let mut builder = Builder::default();
    builder.push_record(["ID", "Commit", "Status", "Created"]);
    for deployment in deployments {
        builder.push_record([
            deployment.id.to_string(),
            deployment.commit_hash.chars().take(8).collect::<String>(),
            deployment.status.to_string(),
            deployment.created_at.format("%Y-%m-%d %H:%M").to_string(),
        ]);
    }
    let mut table = builder.build();
    table.with(Style::rounded());
    println!("{table}");
}

/// Render a single deployment's details as a plain key/value block.
pub fn render_deployment_detail(deployment: &Deployment) {
    println!();
    println!("  {}", highlight(&format!("Deployment #{}", deployment.id)));
    println!("  {:<16}{}", "Project", deployment.project_id);
    println!("  {:<16}{}", "Commit", deployment.commit_hash);
    println!("  {:<16}{}", "Status", deployment.status);
    field_opt("App", deployment.frontend_app_name.as_deref());
    println!(
        "  {:<16}{}",
        "Created",
        deployment.created_at.format("%Y-%m-%d %H:%M")
    );
    println!();
}

/// Print a `label: value` line only when the optional value is present.
fn field_opt(label: &str, value: Option<&str>) {
    if let Some(value) = value {
        if !value.is_empty() {
            println!("  {:<16}{}", label, value);
        }
    }
}
