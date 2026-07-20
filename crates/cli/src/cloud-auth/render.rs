use console::style;
use smbcloud_model::app_auth::AuthApp;

fn print_heading(title: &str) {
    println!("\n{}", style(title).bold().underlined());
}

fn print_field(label: &str, value: impl AsRef<str>) {
    println!("{} {}", style(format!("{label}:")).cyan(), value.as_ref());
}

fn print_optional_field(label: &str, value: Option<&str>) {
    if let Some(value) = value {
        print_field(label, value);
    }
}

pub(crate) fn print_auth_apps(auth_apps: &[AuthApp]) {
    print_heading("Auth apps");

    if auth_apps.is_empty() {
        println!("No Auth apps found.");
        return;
    }

    for auth_app in auth_apps {
        println!(
            "#{} {} project={}",
            auth_app.id,
            style(&auth_app.name).bold(),
            auth_app.project_id.as_deref().unwrap_or("-")
        );
    }
}

pub(crate) fn print_auth_app_detail(auth_app: &AuthApp) {
    print_heading("Auth app");
    print_field("ID", &auth_app.id);
    print_field("Name", &auth_app.name);
    print_optional_field("Project ID", auth_app.project_id.as_deref());
    print_optional_field("Support email", auth_app.support_email.as_deref());
    print_optional_field("Secret", auth_app.secret.as_deref());
    print_field("Created at", auth_app.created_at.to_rfc3339());
    print_field("Updated at", auth_app.updated_at.to_rfc3339());
}
