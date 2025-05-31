use core::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum_macros::EnumIter;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ErrorResponse {
    Error {
        error_code: ErrorCode,
        message: String,
    },
}

impl Error for ErrorResponse {}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Error, Serialize_repr, Deserialize_repr, Debug, EnumIter)]
#[repr(i32)]
pub enum ErrorCode {
    #[error("Unknown error.")]
    Unknown = 0,
    #[error("Parse error.")]    
    ParseError = 1,
    #[error("Network error.")] 
    NetworkError = 2,
    // Projects
    #[error("Project not found.")] 
    ProjectNotFound = 1000
}

impl ErrorCode {
    /// Cannot expose the ErrorCode enum directly to Ruby,
    /// so we need to get it from i32.
    pub fn from_i32(value: i32) -> Self {
        match value {
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
        }
    }
    
    pub fn rb_constant_name(&self) -> String {
        self.to_string()
    }
}
