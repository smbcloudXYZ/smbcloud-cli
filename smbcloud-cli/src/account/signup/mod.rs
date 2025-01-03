pub mod process;
use std::fmt::Display;

pub use process::*;

#[derive(Debug, Clone, Copy)]
pub enum SignupMethod {
    Email,
    GitHub,
}

impl Display for SignupMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Email => write!(f, "Email and password"),
            Self::GitHub => write!(f, "GitHub"),
        }
    }
}
