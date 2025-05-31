use crate::signup::GithubEmail;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{Display, Formatter};

// smbcloud Users.
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "User: id: {}, email: {}", self.id, self.email)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    pub code: Option<i32>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    id: i32,
    email: String,
    created_at: String,
}

// This is smb authorization model.
#[derive(Debug, Serialize, Deserialize)]
pub struct SmbAuthorization {
    pub message: String,
    pub user: Option<User>,
    pub user_email: Option<GithubEmail>,
    pub user_info: Option<GithubInfo>,
    pub error_code: Option<ErrorCode>,
}

#[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u32)]
pub enum ErrorCode {
    EmailNotFound = 1000,
    EmailUnverified = 1001,
    PasswordNotSet = 1003,
    GithubNotLinked = 1004,
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::EmailNotFound => write!(f, "Email not found."),
            ErrorCode::EmailUnverified => write!(f, "Email not verified."),
            ErrorCode::PasswordNotSet => write!(f, "Password not set."),
            ErrorCode::GithubNotLinked => write!(f, "Github not connected."),
        }
    }
}

impl Copy for ErrorCode {}

impl Clone for ErrorCode {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubInfo {
    pub id: i64,
    pub login: String,
    pub name: String,
    pub avatar_url: String,
    pub html_url: String,
    pub email: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_smb_authorization() {
        let smb_authorization = SmbAuthorization {
            message: "test".to_owned(),
            user: None,
            user_email: None,
            user_info: None,
            error_code: Some(ErrorCode::EmailUnverified),
        };
        let json = json!({
            "message": "test",
            "user": null,
            "user_email": null,
            "user_info": null,
            "error_code": 1001,
        });
        assert_eq!(serde_json::to_value(smb_authorization).unwrap(), json);
    }
}
