use crate::ar_date_format;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::Display;
use tsync::tsync;

/// Personal — bootstrapped on signup, one per user, never created/deleted via
/// the API. Organization — a team workspace created explicitly.
#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
#[tsync]
pub enum TenantKind {
    #[default]
    Personal = 0,
    Organization = 1,
}

impl Display for TenantKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantKind::Personal => write!(f, "personal"),
            TenantKind::Organization => write!(f, "organization"),
        }
    }
}

/// The current user's role within a tenant, as returned alongside the tenant
/// in list/show responses (kept off the bare `Tenant` record itself).
#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[tsync]
pub enum TenantRole {
    Owner = 0,
    Admin = 1,
    Member = 2,
}

impl Display for TenantRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantRole::Owner => write!(f, "owner"),
            TenantRole::Admin => write!(f, "admin"),
            TenantRole::Member => write!(f, "member"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct TenantProject {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct Tenant {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub kind: TenantKind,
    /// The requesting user's role in this tenant.
    pub role: TenantRole,
    pub projects_count: i64,
    pub default_project: Option<TenantProject>,
    /// Whether this is the tenant currently selected for the CLI session.
    #[serde(default)]
    pub current: bool,
    #[serde(with = "ar_date_format")]
    pub created_at: DateTime<Utc>,
}

impl Display for Tenant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {}, Name: {}", self.id, self.name)
    }
}

/// Payload for creating an organization tenant. Personal tenants are
/// bootstrapped on signup and can't be created through this endpoint.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[tsync]
pub struct TenantCreate {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[tsync]
pub struct TenantUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl TenantUpdate {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tenant_create() {
        let tenant_create = TenantCreate {
            name: "Acme".to_owned(),
        };
        let json = json!({ "name": "Acme" });
        assert_eq!(serde_json::to_value(tenant_create).unwrap(), json);
    }

    #[test]
    fn test_tenant_update_is_empty() {
        assert!(TenantUpdate::default().is_empty());
        assert!(!TenantUpdate {
            name: Some("Acme".to_owned())
        }
        .is_empty());
    }
}
