use {
    crate::error_codes::{ErrorCode::UnsupportedRunner, ErrorResponse},
    serde::{Deserialize, Serialize},
    std::{
        fmt::{self, Display, Formatter},
        fs,
        path::PathBuf,
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[tsync::tsync]
pub enum Runner {
    NodeJs,
    Ruby,
    Swift,
}

impl Display for Runner {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[tsync::tsync]
pub enum NodeJsFramework {
    NextJs,
    Astro,
}

#[derive(Debug, Serialize, Deserialize)]
#[tsync::tsync]
pub enum RubyFramework {
    Rails,
}

#[derive(Debug, Serialize, Deserialize)]
#[tsync::tsync]
pub enum SwiftFramework {
    Vapor,
}

impl Runner {
    pub fn from(repo_path: &PathBuf) -> Result<Runner, ErrorResponse> {
        if repo_path.join("package.json").exists()
            && (next_config_exists(repo_path) || astro_config_exists(repo_path))
        {
            return Ok(Runner::NodeJs);
        }

        if repo_path.join("Gemfile").exists() {
            return Ok(Runner::Ruby);
        }
        if repo_path.join("Package.swift").exists() {
            return Ok(Runner::Swift);
        }
        Err(ErrorResponse::Error {
            error_code: UnsupportedRunner,
            message: UnsupportedRunner.message(None).to_string(),
        })
    }

    pub fn git_host(&self) -> String {
        format!("git@{}.smbcloud.xyz", self.api())
    }

    fn api(&self) -> &str {
        match self {
            Runner::NodeJs => "api",
            Runner::Ruby | Runner::Swift => "api-1",
        }
    }
}

// Helper function to detect any next.config.* file
fn next_config_exists(repo_path: &PathBuf) -> bool {
    if let Ok(entries) = fs::read_dir(repo_path) {
        for entry in entries.flatten() {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            if filename_str.starts_with("next.config.") {
                return true;
            }
        }
    }
    false
}

// Helper function to detect any astro.config.* file
fn astro_config_exists(repo_path: &PathBuf) -> bool {
    if let Ok(entries) = fs::read_dir(repo_path) {
        for entry in entries.flatten() {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            if filename_str.starts_with("astro.config.") {
                return true;
            }
        }
    }
    false
}
