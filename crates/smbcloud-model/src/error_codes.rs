use core::fmt;
use log::debug;
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
    // Generic errors
    #[error("Unknown error.")]
    Unknown = 0,
    #[error("Parse error.")]
    ParseError = 1,
    #[error("Network connectivity error.")]
    NetworkError = 2,
    #[error("Input error")]
    InputError = 3,
    #[error("Missing config file. Please regenerate with 'smb init'.")]
    MissingConfig = 4,
    // #[error("Missing id in repository. Please regenerate with 'smb init'.")]
    // MissingId,
    #[error("Cancel operation.")]
    Cancel = 5,
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
            // Generic errors
            5 => ErrorCode::Cancel,
            4 => ErrorCode::MissingConfig,
            3 => ErrorCode::InputError,
            2 => ErrorCode::ParseError,
            1 => ErrorCode::NetworkError,
            // Fallback
            _ => ErrorCode::Unknown,
        }
    }

    // This could be better.
    pub fn message(&self, l: Option<String>) -> &str {
        debug!("Language code: {:?}, {}", l, self);
        match self {
            ErrorCode::Unknown => "Unknown error.",
            ErrorCode::ProjectNotFound => "Project not found.",
            ErrorCode::ParseError => "Parse error.",
            ErrorCode::NetworkError => {
                "Network error. Please check your internet connection and try again."
            }
            ErrorCode::Unauthorized => "Unauthorized access.",
            ErrorCode::InputError => "Input error.",
            ErrorCode::MissingConfig => "Missing config.",
            ErrorCode::Cancel => "Cancelled operation.",
        }
    }

    pub fn rb_constant_name(&self) -> String {
        // Using IntoStaticStr to get the variant name directly.
        // self.into() will return a &'static str representing the variant name.
        let variant_name_static_str: &'static str = self.into();
        variant_name_static_str.to_string()
    }
}
