#[cfg(debug_assertions)]
pub const GH_OAUTH_CLIENT_ID: &str = "053e152f1b78ecee552b";
#[cfg(not(debug_assertions))]
pub const GH_OAUTH_CLIENT_ID: &str = "bf1f12d97659a6495e43";

pub const GH_OAUTH_REDIRECT_HOST: &str = "http://localhost";
pub const GH_OAUTH_REDIRECT_PORT: &str = "8808";

pub const SMB_CLIENT_ID: &str = "cli";
pub const SMB_CLIENT_SECRET: &str = "secretttttttt";

// Paths
pub const PATH_USERS: &str = "v1/users";
pub const PATH_USERS_PASSWORD: &str = "v1/users/password";
pub const PATH_USERS_SIGN_OUT: &str = "v1/users/sign_out";
pub const PATH_USERS_SIGN_IN: &str = "v1/users/sign_in";
pub const PATH_LINK_GITHUB_ACCOUNT: &str = "v1/link_github_account";
pub const PATH_RESET_PASSWORD_INSTRUCTIONS: &str = "v1/resend_reset_password_instructions";
pub const PATH_RESEND_CONFIRMATION: &str = "v1/resend_confirmation";
pub const PATH_AUTHORIZE: &str = "v1/authorize";