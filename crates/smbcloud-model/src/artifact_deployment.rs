//! Static deploy over HTTPS: the shapes shared by the CLI and the API.
//!
//! The server validates and owns all of this; the client builds the manifest
//! and reads the status back.

use serde::{Deserialize, Serialize};

/// One file in a deployment manifest. `path` is relative, `/`-separated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// `POST /v1/apps/:app_id/artifact_deployments`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateArtifactDeployment {
    pub manifest: Vec<ArtifactFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}

/// An artifact deployment as the API returns it. `missing` and
/// `upload_url_base` are only present on the create response.
#[derive(Debug, Clone, Deserialize)]
pub struct ArtifactDeployment {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub missing: Vec<String>,
    #[serde(default)]
    pub upload_url_base: Option<String>,
    #[serde(default)]
    pub file_count: Option<u64>,
    #[serde(default)]
    pub total_bytes: Option<u64>,
    #[serde(default)]
    pub commit_sha: Option<String>,
}

impl ArtifactDeployment {
    /// `live`, `failed`, `expired` and `superseded` end a deploy's wait.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status.as_str(),
            "live" | "failed" | "expired" | "superseded"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_create_response() {
        let d: ArtifactDeployment = serde_json::from_str(
            r#"{"id":"x","status":"uploading","missing":["ab"],"upload_url_base":"https://h/blobs","url":null}"#,
        )
        .unwrap();
        assert_eq!(d.missing, ["ab"]);
        assert!(!d.is_terminal());
    }
}
