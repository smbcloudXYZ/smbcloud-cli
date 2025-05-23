use thiserror::Error;

pub struct RemoteUrl {
    /// The git user. Should be git.
    pub user: String,
    /// The custom hostname ends with .git.
    pub host: String,
    /// The path name of the repo, also ends with .git.
    pub path: String,
}

impl RemoteUrl {
    pub fn from(repo_name: &str) -> Self {
        // Present the user with a message to setup remote deployment
        let remote_format = format!("{}.git", repo_name);
        RemoteUrl {
            host: "api.smbcloud.xyz".to_owned(),
            path: remote_format,
            user: "git".to_owned(),
        }
    }

    /// Returns a `Result<GitUrl>` after normalizing and parsing `url` for metadata
    pub fn parse(url: &str) -> Result<RemoteUrl, RemoteUrlParseError> {
        let segments = url.split(':').collect::<Vec<&str>>();
        // Should have two elements
        if segments.len() != 2 {
            return Err(RemoteUrlParseError::ParseError);
        }
        let path = match segments.last() {
            Some(path) => path.to_owned(),
            None => return Err(RemoteUrlParseError::MissingPath),
        };
        let (user, host) = match segments.first() {
            Some(user_host) => parse_user_host(user_host)?,
            None => return Err(RemoteUrlParseError::MissingUserHost),
        };

        if host != path {
            return Err(RemoteUrlParseError::ParseError);
        }

        Ok(RemoteUrl {
            host: host.to_owned(),
            user: user.to_owned(),
            path: path.to_owned(),
        })
    }
}

fn parse_user_host(s: &str) -> Result<(&str, &str), RemoteUrlParseError> {
    let segments = s.split("@").collect::<Vec<&str>>();
    // Should have two elements
    if segments.len() != 2 {
        return Err(RemoteUrlParseError::MissingUserHost);
    }
    let user = match segments.first() {
        Some(user) => user.to_owned(),
        None => return Err(RemoteUrlParseError::MissingUser),
    };
    let host = match segments.last() {
        Some(host) => host.to_owned(),
        None => return Err(RemoteUrlParseError::MissingHost),
    };
    Ok((user, host))
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum RemoteUrlParseError {
    #[error("Git remote url format is invalid. It should be git@projectname.git:projectname.git")]
    ParseError,
    #[error("Path should be present.")]
    MissingPath,
    #[error("User and host should be present.")]
    MissingUserHost,
    #[error("Hostname should be present.")]
    MissingHost,
    #[error("User should be present.")]
    MissingUser,
}
