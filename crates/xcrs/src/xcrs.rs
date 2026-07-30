use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

pub mod mcp;

const IOS_SIMULATOR_DESTINATION_PREFIX: &str = "platform=iOS Simulator,id=";

pub fn encode_base64(data: impl AsRef<[u8]>) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};

    STANDARD.encode(data)
}

pub fn extract_controlkit_elements(root: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut elements = Vec::new();
    collect_controlkit_elements(root, &mut elements);
    elements
}

fn collect_controlkit_elements(element: &serde_json::Value, elements: &mut Vec<serde_json::Value>) {
    let object = match element.as_object() {
        Some(object) => object,
        None => return,
    };

    let rect = object.get("rect").and_then(serde_json::Value::as_object);
    let has_visible_rect = rect
        .and_then(|rect| {
            Some((
                rect.get("x")?.as_f64()?,
                rect.get("y")?.as_f64()?,
                rect.get("width")?.as_f64()?,
                rect.get("height")?.as_f64()?,
            ))
        })
        .is_some_and(|(x, y, width, height)| x >= 0.0 && y >= 0.0 && width > 0.0 && height > 0.0);
    let has_identity = ["label", "name", "value", "rawIdentifier"]
        .iter()
        .any(|key| {
            object
                .get(*key)
                .and_then(serde_json::Value::as_str)
                .is_some()
        });

    if has_visible_rect && has_identity {
        let keys = [
            "type",
            "label",
            "name",
            "value",
            "placeholderValue",
            "rawIdentifier",
            "rect",
        ];
        let element = keys
            .iter()
            .filter_map(|key| {
                object
                    .get(*key)
                    .map(|value| ((*key).to_string(), value.clone()))
            })
            .collect();
        elements.push(serde_json::Value::Object(element));
    }

    if let Some(children) = object.get("children").and_then(serde_json::Value::as_array) {
        for child in children {
            collect_controlkit_elements(child, elements);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ControlKit {
    client: reqwest::Client,
    base_url: String,
}

impl Default for ControlKit {
    fn default() -> Self {
        Self::new(12004)
    }
}

impl ControlKit {
    pub fn new(port: u16) -> Self {
        Self::with_host("127.0.0.1", port)
    }

    pub fn with_host(host: &str, port: u16) -> Self {
        let host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]")
        } else {
            host.to_string()
        };
        Self {
            client: reqwest::Client::new(),
            base_url: format!("http://{host}:{port}"),
        }
    }

    pub async fn health(&self) -> Result<()> {
        let response = self
            .client
            .get(format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .with_context(|| format!("failed to connect to ControlKit at {}", self.base_url))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "ControlKit health check returned HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let response = self
            .client
            .post(format!("{}/rpc", self.base_url))
            .timeout(Duration::from_secs(30))
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
                "id": 1,
            }))
            .send()
            .await
            .with_context(|| format!("failed to connect to ControlKit at {}", self.base_url))?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .context("failed to decode ControlKit JSON-RPC response")?;

        if !status.is_success() {
            return Err(anyhow!("ControlKit returned HTTP {}: {}", status, body));
        }

        if let Some(error) = body.get("error") {
            return Err(anyhow!("ControlKit method '{method}' failed: {error}"));
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| anyhow!("ControlKit response for '{method}' did not contain a result"))
    }
}

#[derive(Debug, Clone)]
pub struct XcodeCommandLineTools {
    xcrun_path: PathBuf,
    xcodebuild_path: PathBuf,
}

impl Default for XcodeCommandLineTools {
    fn default() -> Self {
        Self {
            xcrun_path: PathBuf::from("xcrun"),
            xcodebuild_path: PathBuf::from("xcodebuild"),
        }
    }
}

impl XcodeCommandLineTools {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_paths(xcrun_path: impl Into<PathBuf>, xcodebuild_path: impl Into<PathBuf>) -> Self {
        Self {
            xcrun_path: xcrun_path.into(),
            xcodebuild_path: xcodebuild_path.into(),
        }
    }

    pub fn simctl(&self) -> Simctl<'_> {
        Simctl { tools: self }
    }

    pub fn xcodebuild(&self) -> Xcodebuild<'_> {
        Xcodebuild { tools: self }
    }

    pub fn devicectl(&self) -> Devicectl<'_> {
        Devicectl { tools: self }
    }

    fn run_xcrun<I, S>(&self, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        run_command(&self.xcrun_path, args)
    }

    fn run_xcodebuild<I, S>(&self, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        run_command(&self.xcodebuild_path, args)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ApplePlatform {
    #[serde(rename = "iOS")]
    Ios,
    #[serde(rename = "tvOS")]
    Tvos,
    #[serde(rename = "macOS")]
    Macos,
    #[serde(rename = "watchOS")]
    Watchos,
    #[serde(rename = "visionOS")]
    Visionos,
    #[serde(rename = "unknown")]
    Unknown,
}

impl ApplePlatform {
    fn from_runtime_identifier(runtime_identifier: &str) -> Self {
        if runtime_identifier.contains(".iOS-") {
            Self::Ios
        } else if runtime_identifier.contains(".tvOS-") {
            Self::Tvos
        } else if runtime_identifier.contains(".macOS-") {
            Self::Macos
        } else if runtime_identifier.contains(".watchOS-") {
            Self::Watchos
        } else if runtime_identifier.contains(".visionOS-") || runtime_identifier.contains(".xrOS-")
        {
            Self::Visionos
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Simulator {
    pub platform: ApplePlatform,
    pub runtime_identifier: String,
    pub name: String,
    pub udid: String,
    pub state: String,
    pub is_available: bool,
}

impl Simulator {
    pub fn is_booted(&self) -> bool {
        self.state == "Booted"
    }
}

#[derive(Debug, Deserialize)]
struct SimctlDeviceList {
    devices: BTreeMap<String, Vec<SimctlDevice>>,
}

#[derive(Debug, Deserialize)]
struct SimctlDevice {
    name: String,
    udid: String,
    state: String,
    #[serde(rename = "isAvailable")]
    is_available: Option<bool>,
}

pub struct Simctl<'a> {
    tools: &'a XcodeCommandLineTools,
}

impl Simctl<'_> {
    pub fn list_simulators(&self) -> Result<Vec<Simulator>> {
        let output = self
            .tools
            .run_xcrun(["simctl", "list", "devices", "--json"])?;
        parse_simctl_devices(&output)
    }

    pub fn find_simulator_by_name(&self, name: &str) -> Result<Simulator> {
        self.list_simulators()?
            .into_iter()
            .find(|simulator| simulator.name == name)
            .ok_or_else(|| anyhow!("iOS simulator named '{name}' was not found"))
    }

    pub fn find_simulator_by_udid(&self, udid: &str) -> Result<Simulator> {
        self.list_simulators()?
            .into_iter()
            .find(|simulator| simulator.udid == udid)
            .ok_or_else(|| anyhow!("iOS simulator with UDID '{udid}' was not found"))
    }

    pub fn boot(&self, udid: &str) -> Result<()> {
        if self.find_simulator_by_udid(udid)?.is_booted() {
            return Ok(());
        }

        self.tools.run_xcrun(["simctl", "boot", udid])?;
        Ok(())
    }

    pub fn shutdown(&self, udid: &str) -> Result<()> {
        if !self.find_simulator_by_udid(udid)?.is_booted() {
            return Ok(());
        }

        self.tools.run_xcrun(["simctl", "shutdown", udid])?;
        Ok(())
    }

    pub fn install_app(&self, udid: &str, app_path: impl AsRef<Path>) -> Result<()> {
        let app_path = app_path.as_ref();
        ensure_path_exists(app_path)?;

        let path = app_path
            .to_str()
            .ok_or_else(|| anyhow!("app path is not valid UTF-8: {}", app_path.display()))?;

        self.tools.run_xcrun(["simctl", "install", udid, path])?;
        Ok(())
    }

    pub fn uninstall_app(&self, udid: &str, bundle_id: &str) -> Result<()> {
        self.tools
            .run_xcrun(["simctl", "uninstall", udid, bundle_id])?;
        Ok(())
    }

    pub fn launch_app(&self, udid: &str, bundle_id: &str) -> Result<()> {
        self.tools
            .run_xcrun(["simctl", "launch", udid, bundle_id])?;
        Ok(())
    }

    pub fn terminate_app(&self, udid: &str, bundle_id: &str) -> Result<()> {
        self.tools
            .run_xcrun(["simctl", "terminate", udid, bundle_id])?;
        Ok(())
    }

    pub fn open_url(&self, udid: &str, url: &str) -> Result<()> {
        self.tools.run_xcrun(["simctl", "openurl", udid, url])?;
        Ok(())
    }

    pub fn screenshot(&self, udid: &str) -> Result<Vec<u8>> {
        let temporary_directory = tempfile::tempdir()?;
        let screenshot_path = temporary_directory.path().join("screenshot.png");
        let screenshot_path_string = screenshot_path
            .to_str()
            .ok_or_else(|| anyhow!("screenshot path is not valid UTF-8"))?;

        self.tools
            .run_xcrun(["simctl", "io", udid, "screenshot", screenshot_path_string])?;

        std::fs::read(&screenshot_path)
            .with_context(|| format!("failed to read {}", screenshot_path.display()))
    }

    pub fn app_container_path(&self, udid: &str, bundle_id: &str) -> Result<PathBuf> {
        let output =
            self.tools
                .run_xcrun(["simctl", "get_app_container", udid, bundle_id, "app"])?;
        let path = output.trim();
        if path.is_empty() {
            return Err(anyhow!(
                "simctl returned an empty app container path for '{bundle_id}'"
            ));
        }

        Ok(PathBuf::from(path))
    }
}

pub struct Devicectl<'a> {
    tools: &'a XcodeCommandLineTools,
}

impl Devicectl<'_> {
    /// Captures a PNG screenshot from a physical device known to CoreDevice.
    /// `device` accepts anything `devicectl --device` does: UDID, ECID,
    /// serial number, user-provided name, or DNS name.
    pub fn screenshot(&self, device: &str) -> Result<Vec<u8>> {
        let temporary_directory = tempfile::tempdir()?;
        let screenshot_path = temporary_directory.path().join("screenshot.png");
        let screenshot_path_string = screenshot_path
            .to_str()
            .ok_or_else(|| anyhow!("screenshot path is not valid UTF-8"))?;

        self.tools.run_xcrun([
            "devicectl",
            "device",
            "capture",
            "screenshot",
            "--device",
            device,
            "--destination",
            screenshot_path_string,
        ])?;

        std::fs::read(&screenshot_path)
            .with_context(|| format!("failed to read {}", screenshot_path.display()))
    }
}

#[derive(Debug, Clone)]
pub enum XcodeProject {
    Project(PathBuf),
    Workspace(PathBuf),
}

#[derive(Debug, Clone)]
pub struct BuildIosApp {
    pub project: XcodeProject,
    pub scheme: String,
    pub simulator_udid: String,
    pub derived_data_path: PathBuf,
    pub configuration: Option<String>,
}

pub struct Xcodebuild<'a> {
    tools: &'a XcodeCommandLineTools,
}

impl Xcodebuild<'_> {
    pub fn build_ios_app(&self, request: &BuildIosApp) -> Result<()> {
        let mut args = Vec::new();
        match &request.project {
            XcodeProject::Project(project_path) => {
                ensure_path_exists(project_path)?;
                args.push("-project".to_string());
                args.push(path_to_string(project_path)?);
            }
            XcodeProject::Workspace(workspace_path) => {
                ensure_path_exists(workspace_path)?;
                args.push("-workspace".to_string());
                args.push(path_to_string(workspace_path)?);
            }
        }

        args.push("-scheme".to_string());
        args.push(request.scheme.clone());
        args.push("-destination".to_string());
        args.push(format!(
            "{IOS_SIMULATOR_DESTINATION_PREFIX}{}",
            request.simulator_udid
        ));
        args.push("-derivedDataPath".to_string());
        args.push(path_to_string(&request.derived_data_path)?);

        if let Some(configuration) = &request.configuration {
            args.push("-configuration".to_string());
            args.push(configuration.clone());
        }

        args.push("build".to_string());

        self.tools.run_xcodebuild(args)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct IosAppTestResult {
    pub simulator: Simulator,
    pub bundle_id: String,
    pub app_container_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct IosAppTest {
    pub simulator_name: Option<String>,
    pub simulator_udid: Option<String>,
    pub app_path: PathBuf,
    pub bundle_id: String,
    pub terminate_before_launch: bool,
}

impl XcodeCommandLineTools {
    pub fn run_ios_app_test(&self, request: &IosAppTest) -> Result<IosAppTestResult> {
        if request.simulator_name.is_none() && request.simulator_udid.is_none() {
            return Err(anyhow!(
                "either simulator_name or simulator_udid must be provided"
            ));
        }

        let simctl = self.simctl();
        let simulator = match (&request.simulator_udid, &request.simulator_name) {
            (Some(udid), _) => simctl.find_simulator_by_udid(udid)?,
            (None, Some(name)) => simctl.find_simulator_by_name(name)?,
            (None, None) => unreachable!("validated above"),
        };

        simctl.boot(&simulator.udid)?;
        simctl.install_app(&simulator.udid, &request.app_path)?;

        if request.terminate_before_launch {
            simctl.terminate_app(&simulator.udid, &request.bundle_id)?;
        }

        simctl.launch_app(&simulator.udid, &request.bundle_id)?;
        let app_container_path = simctl.app_container_path(&simulator.udid, &request.bundle_id)?;

        Ok(IosAppTestResult {
            simulator: simctl.find_simulator_by_udid(&simulator.udid)?,
            bundle_id: request.bundle_id.clone(),
            app_container_path,
        })
    }

    pub fn device_tunnel_address(&self, device_udid: &str) -> Result<String> {
        let output = self.run_xcrun([
            "devicectl",
            "device",
            "info",
            "details",
            "--device",
            device_udid,
        ])?;

        parse_device_tunnel_address(&output)
            .ok_or_else(|| anyhow!("device {device_udid} did not report a tunnel IP address"))
    }

    pub fn build_controlkit_device_runner(
        &self,
        request: &BuildControlKitDeviceRunner,
    ) -> Result<PathBuf> {
        ensure_path_exists(&request.project_path)?;
        let args = [
            "build-for-testing".to_string(),
            "-project".to_string(),
            path_to_string(&request.project_path)?,
            "-scheme".to_string(),
            request.scheme.clone(),
            "-configuration".to_string(),
            request.configuration.clone(),
            "-destination".to_string(),
            format!("id={}", request.device_udid),
            "-derivedDataPath".to_string(),
            path_to_string(&request.derived_data_path)?,
        ];
        self.run_xcodebuild(args)?;

        let products_path = request.derived_data_path.join("Build/Products");
        let entries = std::fs::read_dir(&products_path)
            .with_context(|| format!("failed to read {}", products_path.display()))?;
        let mut test_run_paths = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "xctestrun")
            })
            .collect::<Vec<_>>();
        test_run_paths.sort();

        test_run_paths.into_iter().next().ok_or_else(|| {
            anyhow!(
                "Xcode did not create an .xctestrun file in {}",
                products_path.display()
            )
        })
    }

    pub fn start_controlkit_device_runner(
        &self,
        request: &StartControlKitDeviceRunner,
    ) -> Result<RunningControlKitDeviceRunner> {
        ensure_path_exists(&request.xctestrun_path)?;
        set_xctestrun_environment(
            &request.xctestrun_path,
            "CONTROLKIT_LISTEN_HOST",
            &request.listen_host,
        )?;
        set_xctestrun_environment(
            &request.xctestrun_path,
            "CONTROLKIT_LISTEN_PORT",
            &request.listen_port.to_string(),
        )?;

        let tunnel_host = self.device_tunnel_address(&request.device_udid)?;
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&request.log_path)
            .with_context(|| format!("failed to open {}", request.log_path.display()))?;
        let error_log_file = log_file
            .try_clone()
            .with_context(|| format!("failed to clone {}", request.log_path.display()))?;

        let child = Command::new(&self.xcodebuild_path)
            .arg("test-without-building")
            .arg("-xctestrun")
            .arg(&request.xctestrun_path)
            .arg("-destination")
            .arg(format!("id={}", request.device_udid))
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(error_log_file))
            .spawn()
            .with_context(|| {
                format!(
                    "failed to start ControlKit runner for device {}",
                    request.device_udid
                )
            })?;

        Ok(RunningControlKitDeviceRunner {
            process: child,
            tunnel_host,
            listen_port: request.listen_port,
            log_path: request.log_path.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct BuildControlKitDeviceRunner {
    pub project_path: PathBuf,
    pub scheme: String,
    pub configuration: String,
    pub device_udid: String,
    pub derived_data_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StartControlKitDeviceRunner {
    pub device_udid: String,
    pub xctestrun_path: PathBuf,
    pub listen_host: String,
    pub listen_port: u16,
    pub log_path: PathBuf,
}

#[derive(Debug)]
pub struct RunningControlKitDeviceRunner {
    pub process: Child,
    pub tunnel_host: String,
    pub listen_port: u16,
    pub log_path: PathBuf,
}

fn set_xctestrun_environment(path: &Path, name: &str, value: &str) -> Result<()> {
    let key = format!(":TestConfigurations:0:TestTargets:0:EnvironmentVariables:{name}");
    let set_status = Command::new("/usr/libexec/PlistBuddy")
        .arg("-c")
        .arg(format!("Set {key} {value}"))
        .arg(path)
        .status()
        .with_context(|| format!("failed to update {}", path.display()))?;

    if set_status.success() {
        return Ok(());
    }

    let add_status = Command::new("/usr/libexec/PlistBuddy")
        .arg("-c")
        .arg(format!("Add {key} string {value}"))
        .arg(path)
        .status()
        .with_context(|| format!("failed to update {}", path.display()))?;

    if add_status.success() {
        return Ok(());
    }

    Err(anyhow!(
        "failed to set {name} in XCTest plan {}",
        path.display()
    ))
}

fn parse_device_tunnel_address(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.split_once("Tunnel IP Address:"))
        .map(|(_, address)| address.trim().to_string())
        .filter(|address| !address.is_empty())
}

pub fn parse_simctl_devices(output: &str) -> Result<Vec<Simulator>> {
    let list: SimctlDeviceList =
        serde_json::from_str(output).context("failed to parse simctl device list JSON")?;

    let mut simulators = Vec::new();
    for (runtime_identifier, devices) in list.devices {
        for device in devices {
            simulators.push(Simulator {
                platform: ApplePlatform::from_runtime_identifier(&runtime_identifier),
                runtime_identifier: runtime_identifier.clone(),
                name: device.name,
                udid: device.udid,
                state: device.state,
                is_available: device.is_available.unwrap_or(true),
            });
        }
    }

    Ok(simulators)
}

fn ensure_path_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("path does not exist: {}", path.display()));
    }

    Ok(())
}

fn path_to_string(path: &Path) -> Result<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("path is not valid UTF-8: {}", path.display()))
}

fn run_command<I, S>(program: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run {}", program.display()))?;

    command_output_to_result(program, output)
}

fn command_output_to_result(program: &Path, output: Output) -> Result<String> {
    if output.status.success() {
        return String::from_utf8(output.stdout)
            .with_context(|| format!("{} wrote invalid UTF-8 to stdout", program.display()));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let message = [stderr.trim(), stdout.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if message.is_empty() {
        return Err(anyhow!(
            "{} exited with status {}",
            program.display(),
            output.status
        ));
    }

    Err(anyhow!(
        "{} exited with status {}: {}",
        program.display(),
        output.status,
        message
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simctl_devices() {
        let output = r#"{
          "devices": {
            "com.apple.CoreSimulator.SimRuntime.iOS-26-0": [
              {
                "lastBootedAt": "2026-07-30T08:00:00Z",
                "dataPath": "/tmp/device",
                "dataPathSize": 1,
                "logPath": "/tmp/logs",
                "udid": "00000000-0000-0000-0000-000000000000",
                "isAvailable": true,
                "deviceTypeIdentifier": "com.apple.CoreSimulator.SimDeviceType.iPhone-16",
                "state": "Booted",
                "name": "Test-iOS-26"
              }
            ]
          }
        }"#;

        let devices = parse_simctl_devices(output).expect("devices should parse");

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].platform, ApplePlatform::Ios);
        assert_eq!(devices[0].name, "Test-iOS-26");
        assert_eq!(devices[0].state, "Booted");
        assert!(devices[0].is_booted());
    }

    #[test]
    fn parses_xros_simulator_runtime() {
        let output = r#"{
          "devices": {
            "com.apple.CoreSimulator.SimRuntime.xrOS-26-5": [
              {
                "udid": "00000000-0000-0000-0000-000000000001",
                "isAvailable": true,
                "state": "Shutdown",
                "name": "Apple Vision Pro"
              }
            ]
          }
        }"#;

        let devices = parse_simctl_devices(output).expect("devices should parse");

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].platform, ApplePlatform::Visionos);
        assert_eq!(devices[0].name, "Apple Vision Pro");
    }

    #[test]
    fn parses_macos_runtime() {
        let output = r#"{
          "devices": {
            "com.apple.CoreSimulator.SimRuntime.macOS-26-0": [
              {
                "udid": "00000000-0000-0000-0000-000000000002",
                "isAvailable": true,
                "state": "Booted",
                "name": "Mac"
              }
            ]
          }
        }"#;

        let devices = parse_simctl_devices(output).expect("devices should parse");

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].platform, ApplePlatform::Macos);
    }

    #[test]
    fn formats_ipv6_controlkit_url() {
        let controlkit = ControlKit::with_host("fdb4:e020:7377::1", 12006);
        assert_eq!(controlkit.base_url, "http://[fdb4:e020:7377::1]:12006");
    }

    #[test]
    fn parses_device_tunnel_address() {
        let output = "• Device Name: iPhone\n• Tunnel IP Address: fd55:33ce:ad87::1\n";
        assert_eq!(
            parse_device_tunnel_address(output).as_deref(),
            Some("fd55:33ce:ad87::1")
        );
    }
}
