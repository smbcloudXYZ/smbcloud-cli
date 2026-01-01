use {
    core::fmt,
    log::debug,
    serde::{Deserialize, Serialize},
    serde_repr::{Deserialize_repr, Serialize_repr},
    std::fmt::{Display, Formatter},
    strum_macros::{EnumIter, IntoStaticStr},
    thiserror::Error,
};

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
    #[error("Network error. Please check your internet connection and try again.")]
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
    #[error("Invalid params.")]
    InvalidParams = 101,
    // Account not ready errors.
    #[error("Email not found.")]
    EmailNotFound = 1000,
    #[error("Email unverified.")]
    EmailNotVerified = 1001,
    #[error("Email confirmation failed.")]
    EmailConfirmationFailed = 1002,
    #[error("Password is unset.")]
    PasswordNotSet = 1003,
    #[error("GitHub email is not connected.")]
    GitHubEmailNotConnected = 1004,
    #[error("Email already exists.")]
    EmailAlreadyExist = 1005,
    #[error("Invalid password.")]
    InvalidPassword = 1006,
    // Projects
    #[error("Project not found.")]
    ProjectNotFound = 2000,
    #[error("Runner not supported.")]
    UnsupportedRunner = 2001,
}

impl ErrorCode {
    /// Cannot expose the ErrorCode enum directly to Ruby,
    /// so we need to get it from i32.
    pub fn from_i32(value: i32) -> Self {
        match value {
            // Generic
            100 => ErrorCode::Unauthorized,
            101 => ErrorCode::InvalidParams,
            // Account not ready errors
            1000 => ErrorCode::EmailNotFound,
            1001 => ErrorCode::EmailNotVerified,
            1002 => ErrorCode::EmailConfirmationFailed,
            1003 => ErrorCode::PasswordNotSet,
            1004 => ErrorCode::GitHubEmailNotConnected,
            1005 => ErrorCode::EmailAlreadyExist,
            1006 => ErrorCode::InvalidPassword,
            // Projects
            2000 => ErrorCode::ProjectNotFound, // Projects
            2001 => ErrorCode::UnsupportedRunner,
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
            // Networking
            ErrorCode::ParseError => "Parse error.",
            ErrorCode::NetworkError => {
                "Network error. Please check your internet connection and try again."
            }
            // Generic
            ErrorCode::Unauthorized => "Unauthorized access.",
            ErrorCode::InvalidParams => "Invalid parameters.",
            // Account not ready errors
            ErrorCode::EmailNotFound => "Email not found.",
            ErrorCode::EmailNotVerified => "Email not verified.",
            ErrorCode::EmailConfirmationFailed => "Email confirmation faile.",
            ErrorCode::PasswordNotSet => "Password is not set.",
            ErrorCode::GitHubEmailNotConnected => "GitHub email is not connected.",
            ErrorCode::EmailAlreadyExist => "Email already exists.",
            ErrorCode::InvalidPassword => "Invalid password.",
            // CLI Generic errors
            ErrorCode::InputError => "Input error.",
            ErrorCode::MissingConfig => "Missing config.",
            ErrorCode::Cancel => "Cancelled operation.",
            // Projects
            ErrorCode::ProjectNotFound => "Project not found.",
            ErrorCode::UnsupportedRunner => "Unsupported runner.",
        }
    }

    pub fn rb_constant_name(&self) -> String {
        // Using IntoStaticStr to get the variant name directly.
        // self.into() will return a &'static str representing the variant name.
        let variant_name_static_str: &'static str = self.into();
        variant_name_static_str.to_string()
    }
}
