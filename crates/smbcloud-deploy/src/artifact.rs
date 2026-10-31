//! Static deploy over HTTPS: build a manifest from a directory, then upload it.
//!
//! The client builds, hashes the output and uploads blobs the server does not
//! have yet, using only an app-scoped deploy token. The server picks the
//! target and publishes it; nothing here names a remote path or runs a command
//! on the box.

use crate::{error::DeployError, report::Reporter, transport::Transport};
use anyhow::{anyhow, bail, Context};
use ignore::WalkBuilder;
use reqwest::blocking::{Body, Client};
use sha2::{Digest, Sha256};
use smbcloud_model::artifact_deployment::{
    ArtifactDeployment, ArtifactFile, CreateArtifactDeployment,
};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

/// Server-side quotas, checked here too so a refusal is local and early.
pub const MAX_FILES: usize = 20_000;
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 500 * 1024 * 1024;
const UPLOAD_CONCURRENCY: usize = 4;
const UPLOAD_ATTEMPTS: u32 = 3;
/// Never uploaded, whatever the ignore files say.
const ALWAYS_EXCLUDED_DIRS: [&str; 2] = [".git", ".smb"];
const IGNORE_FILES: [&str; 2] = [".gitignore", ".smbignore"];

/// A file to ship: its manifest entry plus where to read it from.
#[derive(Debug, Clone)]
pub struct ManifestFile {
    pub entry: ArtifactFile,
    pub source: PathBuf,
}

/// Everything under a directory that should be published.
#[derive(Debug, Clone)]
pub struct ArtifactManifest {
    pub files: Vec<ManifestFile>,
}

impl ArtifactManifest {
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.entry.size).sum()
    }

    pub fn entries(&self) -> Vec<ArtifactFile> {
        self.files.iter().map(|f| f.entry.clone()).collect()
    }
}

/// Walks `root`, honouring `.gitignore` and `.smbignore` found inside it (not
/// in parent directories), always skipping `.git` and `.smb`, and refusing
/// symlinks and special files. Entries are sorted by path so the manifest is
/// stable.
pub fn build_manifest(root: &Path) -> Result<ArtifactManifest, DeployError> {
    if !root.is_dir() {
        return Err(anyhow!("{} is not a directory", root.display()).into());
    }
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .add_custom_ignore_filename(".smbignore")
        .filter_entry(|entry| {
            let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
            !(is_dir
                && entry.depth() > 0
                && ALWAYS_EXCLUDED_DIRS
                    .iter()
                    .any(|name| entry.file_name() == *name))
        })
        .build();

    let mut files = Vec::new();
    for result in walker {
        let entry = result.map_err(|e| anyhow!("walking {}: {e}", root.display()))?;
        if entry.depth() == 0 {
            continue;
        }
        let file_type = entry
            .file_type()
            .ok_or_else(|| anyhow!("cannot read {}", entry.path().display()))?;
        if file_type.is_dir() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).unwrap_or(entry.path());
        if file_type.is_symlink() {
            return Err(anyhow!(
                "{} is a symlink; symlinks are not published. Remove it or add it to .smbignore",
                relative.display()
            )
            .into());
        }
        if !file_type.is_file() {
            return Err(anyhow!(
                "{} is not a regular file; add it to .smbignore",
                relative.display()
            )
            .into());
        }
        let name = entry.file_name().to_string_lossy();
        if IGNORE_FILES.iter().any(|f| *f == name) {
            continue;
        }

        let path = relative_path(relative)?;
        let (sha256, size) = hash_file(entry.path())
            .with_context(|| format!("hashing {}", relative.display()))
            .map_err(DeployError::Other)?;
        if size > MAX_FILE_BYTES {
            return Err(
                anyhow!("{path} is {size} bytes; the limit is {MAX_FILE_BYTES} per file").into(),
            );
        }
        files.push(ManifestFile {
            entry: ArtifactFile { path, sha256, size },
            source: entry.path().to_path_buf(),
        });
    }

    files.sort_by(|a, b| a.entry.path.cmp(&b.entry.path));
    if files.len() > MAX_FILES {
        return Err(anyhow!("{} files; the limit is {MAX_FILES}", files.len()).into());
    }
    let manifest = ArtifactManifest { files };
    if manifest.total_bytes() > MAX_TOTAL_BYTES {
        return Err(anyhow!(
            "{} bytes in total; the limit is {MAX_TOTAL_BYTES}",
            manifest.total_bytes()
        )
        .into());
    }
    Ok(manifest)
}

fn relative_path(relative: &Path) -> Result<String, DeployError> {
    let mut parts = Vec::new();
    for component in relative.components() {
        let part = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("{} is not valid UTF-8", relative.display()))?;
        if part.contains('\\') || part.chars().any(|c| c.is_control()) {
            return Err(anyhow!(
                "{} has a backslash or control character in its name",
                relative.display()
            )
            .into());
        }
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn hash_file(path: &Path) -> anyhow::Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    let hex = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok((hex, size))
}

/// Uploads a directory as an artifact deployment and waits for it to go live.
///
/// Blocking on purpose, like the rest of the [`Transport`] trait: async
/// callers run `ship` on a blocking thread.
pub struct HttpsArtifactTransport {
    api_base: String,
    app_id: String,
    token: String,
    commit_sha: Option<String>,
    poll_interval: Duration,
    publish_timeout: Duration,
    retry_delay: Duration,
    deployment: Mutex<Option<ArtifactDeployment>>,
}

impl HttpsArtifactTransport {
    /// `api_base` is the scheme and host, e.g. `https://api.smbcloud.xyz`.
    pub fn new(
        api_base: String,
        app_id: String,
        token: String,
        commit_sha: Option<String>,
    ) -> Self {
        Self {
            api_base: api_base.trim_end_matches('/').to_string(),
            app_id,
            token,
            commit_sha,
            poll_interval: Duration::from_secs(2),
            publish_timeout: Duration::from_secs(600),
            retry_delay: Duration::from_millis(500),
            deployment: Mutex::new(None),
        }
    }

    /// Shortens the waits; used by tests.
    pub fn with_timing(
        mut self,
        poll: Duration,
        publish_timeout: Duration,
        retry: Duration,
    ) -> Self {
        self.poll_interval = poll;
        self.publish_timeout = publish_timeout;
        self.retry_delay = retry;
        self
    }

    /// The deployment as last seen, available after `ship`.
    pub fn deployment(&self) -> Option<ArtifactDeployment> {
        self.deployment.lock().unwrap().clone()
    }

    fn url(&self, tail: &str) -> String {
        format!(
            "{}/v1/apps/{}/artifact_deployments{tail}",
            self.api_base, self.app_id
        )
    }

    fn create(
        &self,
        http: &Client,
        manifest: &ArtifactManifest,
    ) -> anyhow::Result<ArtifactDeployment> {
        let body = CreateArtifactDeployment {
            manifest: manifest.entries(),
            commit_sha: self.commit_sha.clone(),
        };
        let response = http
            .post(self.url(""))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .context("creating the deployment")?;
        read_json(response, "creating the deployment")
    }

    fn upload(
        &self,
        http: &Client,
        id: &str,
        sha256: &str,
        source: &Path,
        size: u64,
    ) -> anyhow::Result<()> {
        let mut last = anyhow!("upload never attempted");
        for attempt in 1..=UPLOAD_ATTEMPTS {
            let file =
                File::open(source).with_context(|| format!("opening {}", source.display()))?;
            let result = http
                .put(self.url(&format!("/{id}/blobs/{sha256}")))
                .bearer_auth(&self.token)
                .header("Content-Type", "application/octet-stream")
                .body(Body::sized(file, size))
                .send();
            match result {
                Ok(r) if r.status().is_success() => return Ok(()),
                Ok(r) => {
                    let status = r.status();
                    let detail = r.text().unwrap_or_default();
                    last = anyhow!("upload of {sha256} failed with {status}: {detail}");
                    // A client error will not change on retry, except throttling.
                    if status.is_client_error() && status.as_u16() != 429 {
                        return Err(last);
                    }
                }
                Err(e) => last = anyhow!("upload of {sha256} failed: {e}"),
            }
            if attempt < UPLOAD_ATTEMPTS {
                std::thread::sleep(self.retry_delay * attempt);
            }
        }
        Err(last)
    }

    fn upload_missing(
        &self,
        http: &Client,
        id: &str,
        manifest: &ArtifactManifest,
        missing: &[String],
        reporter: &dyn Reporter,
    ) -> anyhow::Result<()> {
        // One upload per distinct hash, from the first file that has it.
        let mut jobs: Vec<&ManifestFile> = Vec::new();
        for sha in missing {
            if let Some(file) = manifest.files.iter().find(|f| &f.entry.sha256 == sha) {
                jobs.push(file);
            }
        }
        let next = AtomicUsize::new(0);
        let failure: Mutex<Option<anyhow::Error>> = Mutex::new(None);
        std::thread::scope(|scope| {
            for _ in 0..UPLOAD_CONCURRENCY.min(jobs.len()) {
                scope.spawn(|| loop {
                    if failure.lock().unwrap().is_some() {
                        return;
                    }
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(file) = jobs.get(index) else { return };
                    match self.upload(http, id, &file.entry.sha256, &file.source, file.entry.size) {
                        Ok(()) => reporter.info(&format!("uploaded {}", file.entry.path)),
                        Err(e) => {
                            failure.lock().unwrap().get_or_insert(e);
                            return;
                        }
                    }
                });
            }
        });
        match failure.into_inner().unwrap() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn finalize(&self, http: &Client, id: &str) -> anyhow::Result<ArtifactDeployment> {
        let response = http
            .post(self.url(&format!("/{id}/finalize")))
            .bearer_auth(&self.token)
            .send()
            .context("finalizing the deployment")?;
        read_json(response, "finalizing the deployment")
    }

    fn wait_until_published(&self, http: &Client, id: &str) -> anyhow::Result<ArtifactDeployment> {
        let started = Instant::now();
        loop {
            let response = http
                .get(self.url(&format!("/{id}")))
                .bearer_auth(&self.token)
                .send()
                .context("reading the deployment status")?;
            let deployment: ArtifactDeployment =
                read_json(response, "reading the deployment status")?;
            if deployment.is_terminal() {
                return Ok(deployment);
            }
            if started.elapsed() > self.publish_timeout {
                bail!(
                    "the deployment is still {} after {}s",
                    deployment.status,
                    self.publish_timeout.as_secs()
                );
            }
            std::thread::sleep(self.poll_interval);
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(
    response: reqwest::blocking::Response,
    what: &str,
) -> anyhow::Result<T> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        bail!("{what} was refused ({status}): {body}");
    }
    response
        .json()
        .with_context(|| format!("parsing the response from {what}"))
}

impl Transport for HttpsArtifactTransport {
    fn ship(&self, source: &Path, reporter: &dyn Reporter) -> Result<(), DeployError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| anyhow!("building the HTTP client: {e}"))?;

        reporter.step_start(&format!("Hashing {}", source.display()));
        let manifest = build_manifest(source)?;
        reporter.step_done(&format!(
            "{} files, {} bytes",
            manifest.files.len(),
            manifest.total_bytes()
        ));

        let created = self.create(&http, &manifest)?;
        *self.deployment.lock().unwrap() = Some(created.clone());
        reporter.step_start(&format!("Uploading {} new blob(s)", created.missing.len()));
        self.upload_missing(&http, &created.id, &manifest, &created.missing, reporter)?;
        reporter.step_done("Upload complete");

        let queued = self.finalize(&http, &created.id)?;
        *self.deployment.lock().unwrap() = Some(queued);
        reporter.step_start("Publishing");
        let finished = self.wait_until_published(&http, &created.id)?;
        *self.deployment.lock().unwrap() = Some(finished.clone());
        match finished.status.as_str() {
            "live" => {
                reporter.step_done("Live");
                Ok(())
            }
            other => {
                let reason = finished.error.unwrap_or_else(|| other.to_string());
                reporter.step_fail(&reason);
                Err(anyhow!("the deployment ended as {other}: {reason}").into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::NoopReporter;
    use std::fs;
    use wiremock::{
        matchers::{header, method, path, path_regex},
        Mock, MockServer, Request, Respond, ResponseTemplate,
    };

    fn write(dir: &Path, rel: &str, data: &str) {
        let target = dir.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, data).unwrap();
    }

    fn paths(manifest: &ArtifactManifest) -> Vec<String> {
        manifest
            .files
            .iter()
            .map(|f| f.entry.path.clone())
            .collect()
    }

    #[test]
    fn honours_ignore_files_and_always_skips_git_and_smb() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "index.html", "hi");
        write(dir.path(), "assets/app.js", "js");
        write(dir.path(), ".well-known/security.txt", "x");
        write(dir.path(), "secret.env", "s");
        write(dir.path(), "notes/DEPLOY.md", "d");
        write(dir.path(), ".git/config", "g");
        write(dir.path(), ".smb/config.toml", "c");
        write(dir.path(), "sub/.git/HEAD", "g");
        write(dir.path(), ".gitignore", "secret.env\n");
        write(dir.path(), ".smbignore", "notes/\n");

        let manifest = build_manifest(dir.path()).unwrap();

        assert_eq!(
            paths(&manifest),
            [".well-known/security.txt", "assets/app.js", "index.html"]
        );
    }

    #[test]
    fn parent_gitignore_does_not_hide_the_output_dir() {
        let repo = tempfile::tempdir().unwrap();
        write(repo.path(), ".gitignore", "dist/\n");
        write(repo.path(), "dist/index.html", "hi");

        let manifest = build_manifest(&repo.path().join("dist")).unwrap();

        assert_eq!(paths(&manifest), ["index.html"]);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "index.html", "hi");
        std::os::unix::fs::symlink("/etc/passwd", dir.path().join("link")).unwrap();

        let error = build_manifest(dir.path()).unwrap_err().to_string();

        assert!(error.contains("symlink"), "{error}");
    }

    #[test]
    fn hashes_are_stable_and_correct() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", "hello");

        let first = build_manifest(dir.path()).unwrap();
        let second = build_manifest(dir.path()).unwrap();

        assert_eq!(
            first.files[0].entry.sha256,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(first.files[0].entry, second.files[0].entry);
        assert_eq!(first.total_bytes(), 5);
    }

    struct Statuses(Mutex<Vec<&'static str>>);

    impl Respond for Statuses {
        fn respond(&self, _: &Request) -> ResponseTemplate {
            let mut list = self.0.lock().unwrap();
            let status = if list.len() > 1 {
                list.remove(0)
            } else {
                list[0]
            };
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "d1", "status": status, "error": if status == "failed" { Some("disk full") } else { None },
                "url": "https://site.example"
            }))
        }
    }

    fn transport(server: &MockServer) -> HttpsArtifactTransport {
        HttpsArtifactTransport::new(
            server.uri(),
            "app1".into(),
            "smb_deploy_x".into(),
            Some("abc1234".into()),
        )
        .with_timing(
            Duration::from_millis(5),
            Duration::from_secs(5),
            Duration::from_millis(1),
        )
    }

    async fn ship(
        transport: HttpsArtifactTransport,
        dir: PathBuf,
    ) -> (HttpsArtifactTransport, Result<(), DeployError>) {
        tokio::task::spawn_blocking(move || {
            let result = transport.ship(&dir, &NoopReporter);
            (transport, result)
        })
        .await
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uploads_only_the_missing_blobs_then_waits_for_live() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", "hello");
        write(dir.path(), "b.txt", "world");
        let known = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        let new = "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7";

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/app1/artifact_deployments"))
            .and(header("authorization", "Bearer smb_deploy_x"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "d1", "status": "uploading", "missing": [new]
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path(format!(
                "/v1/apps/app1/artifact_deployments/d1/blobs/{new}"
            )))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path(format!(
                "/v1/apps/app1/artifact_deployments/d1/blobs/{known}"
            )))
            .respond_with(ResponseTemplate::new(204))
            .expect(0)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/app1/artifact_deployments/d1/finalize"))
            .respond_with(
                ResponseTemplate::new(202)
                    .set_body_json(serde_json::json!({"id":"d1","status":"queued"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/apps/app1/artifact_deployments/d1"))
            .respond_with(Statuses(Mutex::new(vec!["queued", "publishing", "live"])))
            .mount(&server)
            .await;

        let (transport, result) = ship(transport(&server), dir.path().to_path_buf()).await;

        result.unwrap();
        assert_eq!(
            transport.deployment().unwrap().url.as_deref(),
            Some("https://site.example")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn retries_a_flaky_upload() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", "hello");
        let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/app1/artifact_deployments"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(
                    serde_json::json!({"id":"d1","status":"uploading","missing":[sha]}),
                ),
            )
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path_regex("/blobs/"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path_regex("/blobs/"))
            .respond_with(ResponseTemplate::new(204))
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/app1/artifact_deployments/d1/finalize"))
            .respond_with(
                ResponseTemplate::new(202)
                    .set_body_json(serde_json::json!({"id":"d1","status":"queued"})),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/apps/app1/artifact_deployments/d1"))
            .respond_with(Statuses(Mutex::new(vec!["live"])))
            .mount(&server)
            .await;

        let (_, result) = ship(transport(&server), dir.path().to_path_buf()).await;

        result.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_failed_status_surfaces_the_server_error() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", "hello");
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/app1/artifact_deployments"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(
                    serde_json::json!({"id":"d1","status":"uploading","missing":[]}),
                ),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/app1/artifact_deployments/d1/finalize"))
            .respond_with(
                ResponseTemplate::new(202)
                    .set_body_json(serde_json::json!({"id":"d1","status":"queued"})),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/apps/app1/artifact_deployments/d1"))
            .respond_with(Statuses(Mutex::new(vec!["publishing", "failed"])))
            .mount(&server)
            .await;

        let (_, result) = ship(transport(&server), dir.path().to_path_buf()).await;

        let error = result.unwrap_err().to_string();
        assert!(error.contains("disk full"), "{error}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_refused_create_is_an_error_and_uploads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", "hello");
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/app1/artifact_deployments"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let (_, result) = ship(transport(&server), dir.path().to_path_buf()).await;

        assert!(result.unwrap_err().to_string().contains("401"));
    }
}
