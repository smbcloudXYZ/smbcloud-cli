use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Args {
    pub user: Email,
}

#[derive(Debug, Serialize)]
pub struct Email {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct Param {
    pub user: UserUpdatePassword,
}

#[derive(Debug, Serialize)]
pub struct UserUpdatePassword {
    pub reset_password_token: String,
    pub password: String,
    pub password_confirmation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_update_password() {
        let args = Args {
            user: Email {
                email: "test".to_owned(),
            },
        };
        let json = json!({
            "user": {
                "email": "test",
            },
        });
        assert_eq!(serde_json::to_value(args).unwrap(), json);
    }
}
