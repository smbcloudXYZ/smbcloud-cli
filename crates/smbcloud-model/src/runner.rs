use {
    crate::error_codes::{ErrorCode::UnsupportedRunner, ErrorResponse},
    serde::{Deserialize, Serialize},
    serde_repr::{Deserialize_repr, Serialize_repr},
    std::{
        fmt::{self, Display, Formatter},
        fs,
        path::PathBuf,
    },
};

#[derive(Debug, Deserialize_repr, Serialize_repr, Clone, Copy, PartialEq, Default)]
#[repr(u8)]
#[tsync::tsync]
pub enum Runner {
    #[default]
    NodeJs = 0,
    /// A pure static site: no app process on the server, nginx serves files
    /// directly. Always deployed via rsync — git push has no build step to run.
    Static = 1,
    Ruby = 2,
    Swift = 3,
    Monorepo = 255,
}

impl Display for Runner {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Runner::NodeJs => write!(f, "NodeJs"),
            Runner::Static => write!(f, "Static"),
            Runner::Ruby => write!(f, "Ruby"),
            Runner::Swift => write!(f, "Swift"),
            Runner::Monorepo => write!(f, "Monorepo"),
        }
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
        // See if we have a framework-based config.
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
        // See if we have a monorepo setup.
        non_framework_runner()
    }

    pub fn git_host(&self) -> String {
        format!("git@{}.smbcloud.xyz", self.api())
    }

    /// Returns the explicit hostname used for rsync SSH connections.
    /// e.g. `api.smbcloud.xyz` or `api-1.smbcloud.xyz`
    pub fn rsync_host(&self) -> String {
        format!("{}.smbcloud.xyz", self.api())
    }

    fn api(&self) -> &str {
        match self {
            Runner::Monorepo => "monorepo",
            // Static sites and NodeJs projects share the same lightweight tier
            Runner::NodeJs | Runner::Static => "api",
            Runner::Ruby | Runner::Swift => "api-1",
        }
    }
}

fn non_framework_runner() -> Result<Runner, ErrorResponse> {
    Err(ErrorResponse::Error {
        error_code: UnsupportedRunner,
        message: UnsupportedRunner.message(None).to_string(),
    })
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
