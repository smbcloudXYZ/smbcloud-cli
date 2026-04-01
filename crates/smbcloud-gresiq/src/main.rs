use gresiq::GresiqConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = GresiqConfig::from_env()?;

    println!("GresIQ bootstrap");
    println!("  branch: {}", config.branch_target.summary());
    println!("  compute: {}", config.connection_policy.compute_tier);
    println!("  endpoint: {}", config.redacted_connection_string());

    let connection = config.connect().await?;
    let health_check = connection.health_check().await?;

    println!("  database: {}", health_check.database_name);
    println!("  user: {}", health_check.username);
    println!("  server: {}", health_check.server_version);

    Ok(())
}
