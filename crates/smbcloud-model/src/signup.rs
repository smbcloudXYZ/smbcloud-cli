use crate::account::{Data, Status};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub struct SignupArgs {
    pub email: String,
    pub password: Option<String>,
    pub password_confirmation: Option<String>,
    pub authorizations_attributes: Vec<Provider>,
}

#[derive(Debug, Serialize)]
pub struct Provider {
    pub uid: String,
    pub provider: i8,
}

#[derive(Debug, Serialize)]
pub struct SignupGithubParams {
    pub user: SignupUserGithub,
}

#[derive(Debug, Serialize)]
pub struct SignupEmailParams {
    pub user: SignupUserEmail,
}

#[derive(Debug, Serialize)]
pub struct SignupUserGithub {
    pub email: String,
    pub authorizations_attributes: Vec<Provider>,
}

#[derive(Debug, Serialize)]
pub struct SignupUserEmail {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignupResult {
    pub status: Status,
    pub data: Option<Data>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubUser {
    pub email: Option<String>,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubEmail {
    pub email: String,
    primary: bool,
    verified: bool,
    visibility: Option<String>,
}

impl Display for GithubEmail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.email)
    }
}

#[cfg(test)]
mod tests {}
