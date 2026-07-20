use console::style;
use smbcloud_model::tenant::Tenant;

fn print_heading(title: &str) {
    println!("\n{}", style(title).bold().underlined());
}

fn print_field(label: &str, value: impl AsRef<str>) {
    println!("{} {}", style(format!("{label}:")).cyan(), value.as_ref());
}

pub(crate) fn print_tenants(tenants: &[Tenant]) {
    print_heading("Tenants");

    if tenants.is_empty() {
        println!("No tenants found.");
        return;
    }

    for tenant in tenants {
        let marker = if tenant.current { "*" } else { " " };
        println!(
            "{}#{} {} [{}] slug={} role={} projects={}",
            marker,
            tenant.id,
            style(&tenant.name).bold(),
            tenant.kind,
            tenant.slug,
            tenant.role,
            tenant.projects_count
        );
    }
}

pub(crate) fn print_tenant_detail(tenant: &Tenant) {
    print_heading("Tenant");
    print_field("ID", tenant.id.to_string());
    print_field("Name", &tenant.name);
    print_field("Slug", &tenant.slug);
    print_field("Kind", tenant.kind.to_string());
    print_field("Your role", tenant.role.to_string());
    print_field("Projects", tenant.projects_count.to_string());
    if let Some(default_project) = &tenant.default_project {
        print_field(
            "Default project",
            format!("#{} {}", default_project.id, default_project.name),
        );
    }
    print_field("Created at", tenant.created_at.to_rfc3339());
}
