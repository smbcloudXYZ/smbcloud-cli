use {
    crate::account::{Data, ErrorCode, Status},
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize)]
pub struct LoginArgs {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginParams {
    pub user: UserParam,
}

#[derive(Debug, Serialize)]
pub struct UserParam {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResult {
    pub status: Status,
    pub data: Data,
}

/// Login endpoint result.
#[derive(Debug, Serialize, Deserialize)]
pub enum AccountStatus {
    NotFound,
    Ready { access_token: String },
    Incomplete { status: ErrorCode },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_login() {
        let args = LoginArgs {
            username: "test".to_owned(),
            password: "test".to_owned(),
        };
        let json = json!({
            "username": "test",
            "password": "test",
        });
        assert_eq!(serde_json::to_value(args).unwrap(), json);
    }
}
