use core::fmt;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{Display, Formatter};
use strum_macros::{EnumIter, IntoStaticStr};
use thiserror::Error;

#[derive(Error, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ErrorResponse {
    Error {
        error_code: ErrorCode,
        message: String,
    },
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Error, Serialize_repr, Deserialize_repr, Debug, EnumIter, IntoStaticStr)]
#[repr(i32)]
pub enum ErrorCode {
    #[error("Unknown error.")]
    Unknown = 0,
    #[error("Parse error.")]
    ParseError = 1,
    #[error("Network error.")]
    NetworkError = 2,
    // Account
    #[error("Unauthorized access.")]
    Unauthorized = 100,
    // Projects
    #[error("Project not found.")]
    ProjectNotFound = 1000,
}

impl ErrorCode {
    /// Cannot expose the ErrorCode enum directly to Ruby,
    /// so we need to get it from i32.
    pub fn from_i32(value: i32) -> Self {
        match value {
            // Account
            100 => ErrorCode::Unauthorized,
            // Projects
            1000 => ErrorCode::ProjectNotFound,
            // Fallback
            2 => ErrorCode::ParseError,
            1 => ErrorCode::NetworkError,
            _ => ErrorCode::Unknown,
        }
    }

    // This could be better.
    pub fn message(&self, l: Option<String>) -> &str {
        print!("Language code: {:?}, {}", l, self);
        match self {
            ErrorCode::Unknown => "Unknown error.",
            ErrorCode::ProjectNotFound => "Project not found.",
            ErrorCode::ParseError => "Parse error.",
            ErrorCode::NetworkError => "Network error.",
            ErrorCode::Unauthorized => "Unauthorized access.",
        }
    }

    pub fn rb_constant_name(&self) -> String {
        // Using IntoStaticStr to get the variant name directly.
        // self.into() will return a &'static str representing the variant name.
        let variant_name_static_str: &'static str = self.into();
        variant_name_static_str.to_string()
    }
}
