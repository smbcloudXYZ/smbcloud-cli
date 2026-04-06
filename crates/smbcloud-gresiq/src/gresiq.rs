pub mod rest;

use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use std::{env, fmt, num::ParseIntError};
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio_postgres::{
    Client, Config as PostgresConfig, Error as PostgresError, NoTls, Row,
    config::SslMode as PostgresSslMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GresiqEnvironment {
    Development,
    Preview,
    Production,
}

impl GresiqEnvironment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Preview => "preview",
            Self::Production => "production",
        }
    }
}

impl fmt::Display for GresiqEnvironment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for GresiqEnvironment {
    type Err = GresiqConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "preview" | "staging" => Ok(Self::Preview),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(GresiqConfigError::InvalidEnvironment(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeTier {
    Burst,
    Autoscale,
    Dedicated,
}

impl ComputeTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Burst => "burst",
            Self::Autoscale => "autoscale",
            Self::Dedicated => "dedicated",
        }
    }
}

impl fmt::Display for ComputeTier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchTarget {
    pub project_slug: String,
    pub branch_name: String,
    pub region: String,
    pub environment: GresiqEnvironment,
}

impl BranchTarget {
    pub fn summary(&self) -> String {
        format!(
            "{}/{}/{} ({})",
            self.project_slug, self.branch_name, self.region, self.environment
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub database_name: String,
}

impl Credentials {
    pub fn redacted_password(&self) -> &'static str {
        "********"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub ssl_mode: SslMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SslMode {
    Disable,
    Prefer,
    Require,
}

impl SslMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disable => "disable",
            Self::Prefer => "prefer",
            Self::Require => "require",
        }
    }
}

impl fmt::Display for SslMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for SslMode {
    type Err = GresiqConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "disable" => Ok(Self::Disable),
            "prefer" => Ok(Self::Prefer),
            "require" => Ok(Self::Require),
            _ => Err(GresiqConfigError::InvalidSslMode(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionPolicy {
    pub compute_tier: ComputeTier,
    pub application_name: String,
    pub connect_timeout_seconds: u64,
}

impl Default for ConnectionPolicy {
    fn default() -> Self {
        Self {
            compute_tier: ComputeTier::Autoscale,
            application_name: "gresiq".to_owned(),
            connect_timeout_seconds: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GresiqConfig {
    pub branch_target: BranchTarget,
    pub endpoint: Endpoint,
    pub credentials: Credentials,
    pub connection_policy: ConnectionPolicy,
}

impl GresiqConfig {
    pub fn from_env() -> Result<Self, GresiqConfigError> {
        let environment = env::var("SMBCLOUD_ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_owned())
            .parse()?;

        let ssl_mode = env::var("GRESIQ_SSL_MODE")
            .unwrap_or_else(|_| "require".to_owned())
            .parse()?;

        let port = read_optional_port("GRESIQ_PORT")?.unwrap_or(5432);

        let connect_timeout_seconds =
            read_optional_u64("GRESIQ_CONNECT_TIMEOUT_SECONDS")?.unwrap_or(10);

        let compute_tier = match env::var("GRESIQ_COMPUTE_TIER")
            .unwrap_or_else(|_| "autoscale".to_owned())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "burst" => ComputeTier::Burst,
            "autoscale" => ComputeTier::Autoscale,
            "dedicated" => ComputeTier::Dedicated,
            other => {
                return Err(GresiqConfigError::InvalidComputeTier(other.to_owned()));
            }
        };

        Ok(Self {
            branch_target: BranchTarget {
                project_slug: read_required_env("GRESIQ_PROJECT_SLUG")?,
                branch_name: env::var("GRESIQ_BRANCH").unwrap_or_else(|_| "main".to_owned()),
                region: env::var("GRESIQ_REGION").unwrap_or_else(|_| "eu-north-1".to_owned()),
                environment,
            },
            endpoint: Endpoint {
                host: read_required_env("GRESIQ_HOST")?,
                port,
                ssl_mode,
            },
            credentials: Credentials {
                username: read_required_env("GRESIQ_USERNAME")?,
                password: read_required_env("GRESIQ_PASSWORD")?,
                database_name: env::var("GRESIQ_DATABASE")
                    .unwrap_or_else(|_| "postgres".to_owned()),
            },
            connection_policy: ConnectionPolicy {
                compute_tier,
                application_name: env::var("GRESIQ_APPLICATION_NAME")
                    .unwrap_or_else(|_| "gresiq".to_owned()),
                connect_timeout_seconds,
            },
        })
    }

    pub fn connection_string(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}?sslmode={}&application_name={}",
            self.credentials.username,
            self.credentials.password,
            self.endpoint.host,
            self.endpoint.port,
            self.credentials.database_name,
            self.endpoint.ssl_mode,
            self.connection_policy.application_name
        )
    }

    pub fn redacted_connection_string(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}?sslmode={}&application_name={}",
            self.credentials.username,
            self.credentials.redacted_password(),
            self.endpoint.host,
            self.endpoint.port,
            self.credentials.database_name,
            self.endpoint.ssl_mode,
            self.connection_policy.application_name
        )
    }

    pub fn postgres_config(&self) -> PostgresConfig {
        let mut config = PostgresConfig::new();
        config.host(&self.endpoint.host);
        config.port(self.endpoint.port);
        config.user(&self.credentials.username);
        config.password(&self.credentials.password);
        config.dbname(&self.credentials.database_name);
        config.application_name(&self.connection_policy.application_name);
        config.connect_timeout(std::time::Duration::from_secs(
            self.connection_policy.connect_timeout_seconds,
        ));
        config.ssl_mode(match self.endpoint.ssl_mode {
            SslMode::Disable => PostgresSslMode::Disable,
            SslMode::Prefer => PostgresSslMode::Prefer,
            SslMode::Require => PostgresSslMode::Require,
        });
        config
    }

    pub async fn connect(&self) -> Result<GresiqConnection, GresiqConnectError> {
        let postgres_config = self.postgres_config();

        match self.endpoint.ssl_mode {
            SslMode::Disable => {
                let (client, connection) = postgres_config.connect(NoTls).await?;
                let connection_task = tokio::spawn(async move { connection.await });

                Ok(GresiqConnection {
                    client,
                    connection_task: Some(connection_task),
                })
            }
            SslMode::Prefer | SslMode::Require => {
                let tls_connector = TlsConnector::builder().build()?;
                let tls_connector = MakeTlsConnector::new(tls_connector);
                let (client, connection) = postgres_config.connect(tls_connector).await?;
                let connection_task = tokio::spawn(async move { connection.await });

                Ok(GresiqConnection {
                    client,
                    connection_task: Some(connection_task),
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct GresiqConnection {
    client: Client,
    connection_task: Option<JoinHandle<Result<(), PostgresError>>>,
}

impl GresiqConnection {
    pub fn client(&self) -> &Client {
        &self.client
    }

    pub async fn health_check(&self) -> Result<GresiqHealthCheck, GresiqConnectError> {
        let row = self
            .client
            .query_one("select current_database(), current_user, version()", &[])
            .await?;

        Ok(GresiqHealthCheck::from_row(&row))
    }

    pub async fn close(mut self) -> Result<(), GresiqConnectError> {
        if let Some(connection_task) = self.connection_task.take() {
            connection_task.await??;
        }
        Ok(())
    }
}

impl Drop for GresiqConnection {
    fn drop(&mut self) {
        if let Some(connection_task) = self.connection_task.take() {
            connection_task.abort();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GresiqHealthCheck {
    pub database_name: String,
    pub username: String,
    pub server_version: String,
}

impl GresiqHealthCheck {
    fn from_row(row: &Row) -> Self {
        Self {
            database_name: row.get(0),
            username: row.get(1),
            server_version: row.get(2),
        }
    }
}

#[derive(Debug, Error)]
pub enum GresiqConfigError {
    #[error("missing required environment variable `{0}`")]
    MissingEnvVar(&'static str),
    #[error("environment variable `{0}` contains invalid unicode")]
    InvalidEnvUnicode(&'static str),
    #[error("failed to parse environment variable `{name}` as an integer")]
    InvalidNumber {
        name: &'static str,
        #[source]
        source: ParseIntError,
    },
    #[error("invalid smbCloud environment `{0}`")]
    InvalidEnvironment(String),
    #[error("invalid GresIQ ssl mode `{0}`")]
    InvalidSslMode(String),
    #[error("invalid GresIQ compute tier `{0}`")]
    InvalidComputeTier(String),
}

#[derive(Debug, Error)]
pub enum GresiqConnectError {
    #[error(transparent)]
    Tls(#[from] native_tls::Error),
    #[error(transparent)]
    Postgres(#[from] PostgresError),
    #[error("postgres connection task failed")]
    Join(#[from] tokio::task::JoinError),
}

fn read_required_env(name: &'static str) -> Result<String, GresiqConfigError> {
    env::var(name).map_err(|_| GresiqConfigError::MissingEnvVar(name))
}

fn read_optional_port(name: &'static str) -> Result<Option<u16>, GresiqConfigError> {
    read_optional_env(name)?
        .map(|value| {
            value
                .parse()
                .map_err(|source| GresiqConfigError::InvalidNumber { name, source })
        })
        .transpose()
}

fn read_optional_u64(name: &'static str) -> Result<Option<u64>, GresiqConfigError> {
    read_optional_env(name)?
        .map(|value| {
            value
                .parse()
                .map_err(|source| GresiqConfigError::InvalidNumber { name, source })
        })
        .transpose()
}

fn read_optional_env(name: &'static str) -> Result<Option<String>, GresiqConfigError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(GresiqConfigError::InvalidEnvUnicode(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_connection_string_hides_the_password() {
        let config = GresiqConfig {
            branch_target: BranchTarget {
                project_slug: "acme-store".to_owned(),
                branch_name: "main".to_owned(),
                region: "eu-north-1".to_owned(),
                environment: GresiqEnvironment::Production,
            },
            endpoint: Endpoint {
                host: "main.acme-store.gresiq.smbcloud.xyz".to_owned(),
                port: 5432,
                ssl_mode: SslMode::Require,
            },
            credentials: Credentials {
                username: "postgres".to_owned(),
                password: "super-secret".to_owned(),
                database_name: "app".to_owned(),
            },
            connection_policy: ConnectionPolicy::default(),
        };

        assert!(!config.redacted_connection_string().contains("super-secret"));
        assert!(config.redacted_connection_string().contains("********"));
    }

    #[test]
    fn branch_summary_matches_expected_shape() {
        let branch_target = BranchTarget {
            project_slug: "acme-store".to_owned(),
            branch_name: "preview".to_owned(),
            region: "eu-west-1".to_owned(),
            environment: GresiqEnvironment::Preview,
        };

        assert_eq!(
            branch_target.summary(),
            "acme-store/preview/eu-west-1 (preview)"
        );
    }
}

pub use rest::{GresiqRestClient, GresiqRestConfig, GresiqRestError, OrderDir, QueryBuilder};
