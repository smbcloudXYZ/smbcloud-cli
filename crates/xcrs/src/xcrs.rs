use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

pub mod mcp;

const IOS_SIMULATOR_DESTINATION_PREFIX: &str = "platform=iOS Simulator,id=";
const CONTROLKIT_METHOD_NOT_FOUND: i64 = -32601;
const CONTROLKIT_RUNNER_INFO_TIMEOUT: Duration = Duration::from_secs(2);
const ANDROIDKIT_CALL_TIMEOUT: Duration = Duration::from_secs(10);
const ANDROIDKIT_START_TIMEOUT: Duration = Duration::from_secs(10);
const ANDROIDKIT_START_PROBE_TIMEOUT: Duration = Duration::from_millis(500);
const ANDROIDKIT_START_RETRY_DELAY: Duration = Duration::from_millis(100);

/// `ETXTBSY`, for an executable that another process still holds open for
/// writing. It has the same value on Linux and the BSDs, and `ErrorKind` has no
/// variant for it.
#[cfg(unix)]
const TEXT_FILE_BUSY: i32 = 26;
#[cfg(unix)]
const TEXT_FILE_BUSY_RETRY_DELAY: Duration = Duration::from_millis(20);
#[cfg(unix)]
const TEXT_FILE_BUSY_RETRY_ATTEMPTS: usize = 5;

pub fn encode_base64(data: impl AsRef<[u8]>) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};

    STANDARD.encode(data)
}

pub fn extract_controlkit_elements(root: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut elements = Vec::new();
    collect_controlkit_elements(root, &mut elements);
    elements
}

/// Normalize actionable AndroidKit nodes into the public element-list shape.
pub fn extract_androidkit_elements(root: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut elements = Vec::new();
    if let Some(hierarchy) = root.get("hierarchy").and_then(serde_json::Value::as_array) {
        for element in hierarchy {
            collect_androidkit_elements(element, &mut elements);
        }
    } else {
        collect_androidkit_elements(root, &mut elements);
    }
    elements
}

fn collect_androidkit_elements(element: &serde_json::Value, elements: &mut Vec<serde_json::Value>) {
    let Some(object) = element.as_object() else {
        return;
    };

    let rect = object.get("rect").and_then(serde_json::Value::as_object);
    let has_visible_rect = object
        .get("visible")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && rect
            .and_then(|rect| {
                Some((
                    rect.get("x")?.as_f64()?,
                    rect.get("y")?.as_f64()?,
                    rect.get("width")?.as_f64()?,
                    rect.get("height")?.as_f64()?,
                ))
            })
            .is_some_and(|(_, _, width, height)| width > 0.0 && height > 0.0);
    let text = object
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let description = object
        .get("contentDescription")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let resource_id = object
        .get("resourceId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let hint = object
        .get("hint")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    if has_visible_rect
        && [text, description, resource_id, hint]
            .iter()
            .any(|value| !value.is_empty())
    {
        elements.push(serde_json::json!({
            "type": object.get("role").cloned().unwrap_or(serde_json::Value::Null),
            "label": if !text.is_empty() { text } else { description },
            "name": resource_id,
            "value": text,
            "placeholderValue": hint,
            "rawIdentifier": resource_id,
            "rect": object.get("rect").cloned().unwrap_or(serde_json::Value::Null),
            "enabled": object.get("enabled").cloned().unwrap_or(serde_json::Value::Bool(false)),
            "selected": object.get("selected").cloned().unwrap_or(serde_json::Value::Bool(false)),
            "hittable": object.get("clickable").cloned().unwrap_or(serde_json::Value::Bool(false)),
        }));
    }

    if let Some(children) = object.get("children").and_then(serde_json::Value::as_array) {
        for child in children {
            collect_androidkit_elements(child, elements);
        }
    }
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
                .is_some_and(|value| !value.is_empty())
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
            "depth",
            "enabled",
            "selected",
            "hittable",
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
        let body = self.send(method, params).await?;

        if let Some(error) = body.get("error") {
            // `send` cannot recurse; this guard only avoids repeating an unsupported probe.
            let should_probe_runner =
                is_controlkit_method_not_found(error) && method != "device.info";
            let (runner_info, runner_info_error) = if should_probe_runner {
                match tokio::time::timeout(
                    CONTROLKIT_RUNNER_INFO_TIMEOUT,
                    self.send("device.info", serde_json::json!({})),
                )
                .await
                {
                    Ok(Ok(body)) => match body.get("result") {
                        Some(result) => (Some(result.clone()), None),
                        None => {
                            let reason = body
                                .get("error")
                                .map(|error| format!("device.info returned {error}"))
                                .unwrap_or_else(|| {
                                    "device.info response did not contain a result".to_string()
                                });
                            (None, Some(reason))
                        }
                    },
                    Ok(Err(error)) => (None, Some(error.to_string())),
                    Err(_) => (
                        None,
                        Some(format!(
                            "device.info timed out after {} seconds",
                            CONTROLKIT_RUNNER_INFO_TIMEOUT.as_secs()
                        )),
                    ),
                }
            } else {
                (None, None)
            };
            return Err(anyhow!(controlkit_rpc_error(
                method,
                error,
                runner_info.as_ref(),
                runner_info_error.as_deref()
            )));
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| anyhow!("ControlKit response for '{method}' did not contain a result"))
    }

    async fn send(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
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

        Ok(body)
    }
}

fn is_controlkit_method_not_found(error: &serde_json::Value) -> bool {
    let Some(code) = error.get("code") else {
        return false;
    };

    // Some runners serialise the JSON-RPC code as an integral float (e.g. `-32601.0`),
    // which `as_i64` rejects, so fall back to an exact `as_f64` comparison for that case.
    code.as_i64() == Some(CONTROLKIT_METHOD_NOT_FOUND)
        || code.as_f64() == Some(CONTROLKIT_METHOD_NOT_FOUND as f64)
}

fn controlkit_rpc_error(
    method: &str,
    error: &serde_json::Value,
    runner_info: Option<&serde_json::Value>,
    runner_info_error: Option<&str>,
) -> String {
    let code = error
        .get("code")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let message = error
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown JSON-RPC error");

    if !is_controlkit_method_not_found(error) {
        return format!("ControlKit method '{method}' failed ({code}): {message}");
    }

    let runner = runner_info
        .and_then(|info| info.get("runner"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("The connected ControlKit runner");
    let protocol = runner_info
        .and_then(|info| info.get("protocolVersion"))
        .and_then(serde_json::Value::as_u64)
        .map(|version| format!(" protocol {version}"))
        .unwrap_or_default();

    let runner_info_note = runner_info_error
        .map(|error| format!(" Runner metadata could not be read: {error}."))
        .unwrap_or_default();

    format!(
        "{runner}{protocol} does not implement ControlKit method '{method}'.{runner_info_note} Rebuild or upgrade xcrs-controlkit, restart its runner, and retry."
    )
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
            .ok_or_else(|| anyhow!("Apple simulator named '{name}' was not found"))
    }

    pub fn find_simulator_by_udid(&self, udid: &str) -> Result<Simulator> {
        self.list_simulators()?
            .into_iter()
            .find(|simulator| simulator.udid == udid)
            .ok_or_else(|| anyhow!("Apple simulator with UDID '{udid}' was not found"))
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AndroidDevice {
    pub serial: String,
    pub state: String,
    pub product: Option<String>,
    pub model: Option<String>,
    pub device: Option<String>,
    pub transport_id: Option<String>,
}

impl AndroidDevice {
    pub fn is_connected(&self) -> bool {
        self.state == "device"
    }
}

/// The automation platform a normalized `device_list` entry belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DevicePlatform {
    Apple,
    Android,
}

/// The kind of device a normalized `device_list` entry represents. Apple physical
/// devices are not represented here: this crate has no reliable Apple
/// device-discovery command, so only Apple simulators are ever listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Simulator,
    Emulator,
    PhysicalDevice,
}

/// A device entry normalized across Apple simulators and Android devices/emulators
/// for the cross-platform `device_list` tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedDevice {
    pub platform: DevicePlatform,
    pub kind: DeviceKind,
    /// Simulator UDID (Apple) or device serial (Android).
    pub identifier: String,
    pub name: String,
    pub state: String,
    /// The tool used to reach this device: "simctl" or "adb".
    pub transport: String,
}

impl NormalizedDevice {
    pub fn from_simulator(simulator: &Simulator) -> Self {
        Self {
            platform: DevicePlatform::Apple,
            kind: DeviceKind::Simulator,
            identifier: simulator.udid.clone(),
            name: simulator.name.clone(),
            state: simulator.state.clone(),
            transport: "simctl".to_string(),
        }
    }

    pub fn from_android_device(device: &AndroidDevice) -> Self {
        let kind = if device.serial.starts_with("emulator-") {
            DeviceKind::Emulator
        } else {
            DeviceKind::PhysicalDevice
        };
        let name = device
            .model
            .clone()
            .or_else(|| device.device.clone())
            .or_else(|| device.product.clone())
            .unwrap_or_else(|| device.serial.clone());
        Self {
            platform: DevicePlatform::Android,
            kind,
            identifier: device.serial.clone(),
            name,
            state: device.state.clone(),
            transport: "adb".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AndroidDebugBridge {
    adb_path: PathBuf,
}

/// Client for an XCRS AndroidKit instrumentation runner forwarded over adb.
#[derive(Debug, Clone)]
pub struct AndroidKit {
    client: reqwest::Client,
    adb_path: PathBuf,
}

impl Default for AndroidKit {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
            adb_path: discover_adb_path(),
        }
    }
}

impl AndroidKit {
    pub const PACKAGE_NAME: &'static str = "xyz.smbcloud.xcrs.androidkit";

    pub fn new() -> Self {
        Self::default()
    }

    pub async fn call(
        &self,
        serial: String,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.call_with_timeout(serial, method, params, ANDROIDKIT_CALL_TIMEOUT)
            .await
    }

    fn with_adb_path(adb_path: impl Into<PathBuf>) -> Self {
        Self {
            client: reqwest::Client::new(),
            adb_path: adb_path.into(),
        }
    }

    async fn call_with_timeout(
        &self,
        serial: String,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value> {
        let bridge = AndroidDebugBridge::with_path(self.adb_path.clone());
        let local_port = tokio::task::spawn_blocking({
            let serial = serial.clone();
            move || bridge.forward_androidkit(&serial)
        })
        .await
        .context("AndroidKit forwarding task failed")??;

        let response = self
            .client
            .post(format!("http://127.0.0.1:{local_port}/rpc"))
            .timeout(timeout)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            }))
            .send()
            .await
            .with_context(|| format!("failed to connect to XCRS AndroidKit for {serial}"));

        let cleanup_bridge = AndroidDebugBridge::with_path(self.adb_path.clone());
        let cleanup_serial = serial.clone();
        let cleanup = tokio::task::spawn_blocking(move || {
            cleanup_bridge.remove_forward(&cleanup_serial, local_port)
        })
        .await
        .context("AndroidKit forwarding cleanup task failed")?;
        cleanup.context("failed to remove AndroidKit adb forward")?;

        let response = response?;
        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .context("failed to decode AndroidKit JSON-RPC response")?;
        if !status.is_success() {
            return Err(anyhow!("AndroidKit returned HTTP {status}: {body}"));
        }
        if let Some(error) = body.get("error") {
            let code = error
                .get("code")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            let message = error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown JSON-RPC error");
            return Err(anyhow!(
                "AndroidKit method '{method}' failed ({code}): {message}"
            ));
        }
        body.get("result")
            .cloned()
            .ok_or_else(|| anyhow!("AndroidKit response for '{method}' did not contain a result"))
    }

    async fn wait_until_ready(&self, serial: &str) -> Result<()> {
        let deadline = tokio::time::Instant::now() + ANDROIDKIT_START_TIMEOUT;
        loop {
            match self
                .call_with_timeout(
                    serial.to_string(),
                    "device.info",
                    serde_json::json!({}),
                    ANDROIDKIT_START_PROBE_TIMEOUT,
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(error) if tokio::time::Instant::now() >= deadline => {
                    return Err(error).with_context(|| {
                        format!(
                            "AndroidKit did not become ready on {serial} within {} seconds",
                            ANDROIDKIT_START_TIMEOUT.as_secs()
                        )
                    });
                }
                Err(_) => tokio::time::sleep(ANDROIDKIT_START_RETRY_DELAY).await,
            }
        }
    }
}

impl Default for AndroidDebugBridge {
    fn default() -> Self {
        Self {
            adb_path: discover_adb_path(),
        }
    }
}

impl AndroidDebugBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_path(adb_path: impl Into<PathBuf>) -> Self {
        Self {
            adb_path: adb_path.into(),
        }
    }

    pub fn list_devices(&self) -> Result<Vec<AndroidDevice>> {
        let output = run_command(&self.adb_path, ["devices", "-l"])?;
        parse_adb_devices(&output)
    }

    pub fn resolve_device(&self, serial: Option<&str>) -> Result<AndroidDevice> {
        let devices = self.list_devices()?;
        if let Some(serial) = serial {
            let device = devices
                .into_iter()
                .find(|device| device.serial == serial)
                .ok_or_else(|| anyhow!("Android device '{serial}' was not found"))?;
            if !device.is_connected() {
                return Err(anyhow!(
                    "Android device '{}' is {}",
                    device.serial,
                    device.state
                ));
            }
            return Ok(device);
        }

        let connected_devices = devices
            .into_iter()
            .filter(AndroidDevice::is_connected)
            .collect::<Vec<_>>();
        match connected_devices.as_slice() {
            [device] => Ok(device.clone()),
            [] => Err(anyhow!(
                "no authorized Android device is connected; check `adb devices -l`"
            )),
            _ => Err(anyhow!(
                "multiple Android devices are connected; provide a device serial"
            )),
        }
    }

    pub fn screenshot(&self, serial: &str) -> Result<Vec<u8>> {
        self.run_for_device_bytes(serial, ["exec-out", "screencap", "-p"])
    }

    pub fn launch_app(&self, serial: &str, package_name: &str) -> Result<()> {
        validate_android_package_name(package_name)?;
        let component = [
            "android.intent.category.LEANBACK_LAUNCHER",
            "android.intent.category.LAUNCHER",
        ]
        .into_iter()
        .find_map(|category| {
            let output = self
                .run_shell(
                    serial,
                    &format!(
                        "cmd package resolve-activity --brief -a android.intent.action.MAIN -c {category} {}",
                        shell_escape(package_name)
                    ),
                )
                .ok()?;
            parse_resolved_android_activity(&output)
        })
        .ok_or_else(|| anyhow!("no launchable activity found for Android app '{package_name}'"))?;

        let output = self.run_shell(
            serial,
            &format!("am start -W -n {}", shell_escape(&component)),
        )?;
        if output.lines().any(|line| line.starts_with("Error:")) {
            return Err(anyhow!(
                "failed to launch Android app '{package_name}': {}",
                output.trim()
            ));
        }
        Ok(())
    }

    pub fn terminate_app(&self, serial: &str, package_name: &str) -> Result<()> {
        validate_android_package_name(package_name)?;
        self.run_shell(
            serial,
            &format!("am force-stop {}", shell_escape(package_name)),
        )?;
        Ok(())
    }

    pub fn open_url(&self, serial: &str, url: &str) -> Result<()> {
        if url.is_empty() || url.contains(['\0', '\n', '\r']) {
            return Err(anyhow!("URL must be a non-empty single-line value"));
        }
        self.run_shell(
            serial,
            &format!(
                "am start -a android.intent.action.VIEW -d {}",
                shell_escape(url)
            ),
        )?;
        Ok(())
    }

    pub fn tap(&self, serial: &str, x: i32, y: i32) -> Result<()> {
        self.run_shell(serial, &format!("input tap {x} {y}"))?;
        Ok(())
    }

    pub fn type_text(&self, serial: &str, text: &str) -> Result<()> {
        let text = encode_adb_input_text(text)?;
        self.run_shell(serial, &format!("input text {}", shell_escape(&text)))?;
        Ok(())
    }

    pub fn swipe(
        &self,
        serial: &str,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        duration_ms: u32,
    ) -> Result<()> {
        if duration_ms > 10_000 {
            return Err(anyhow!("swipe duration must not exceed 10000 milliseconds"));
        }
        self.run_shell(
            serial,
            &format!("input swipe {x1} {y1} {x2} {y2} {duration_ms}"),
        )?;
        Ok(())
    }

    pub fn press_button(&self, serial: &str, button: &str) -> Result<()> {
        let keycode = match button {
            "home" => 3,
            "back" => 4,
            "enter" => 66,
            "recents" => 187,
            _ => {
                return Err(anyhow!(
                    "button must be one of home, back, enter, or recents"
                ))
            }
        };
        self.run_shell(serial, &format!("input keyevent {keycode}"))?;
        Ok(())
    }

    pub fn install_androidkit(&self, serial: &str, apk_path: &Path) -> Result<()> {
        if !apk_path.is_file() {
            return Err(anyhow!(
                "AndroidKit APK does not exist: {}",
                apk_path.display()
            ));
        }
        run_command(
            &self.adb_path,
            [
                "-s",
                serial,
                "install",
                "-r",
                apk_path
                    .to_str()
                    .ok_or_else(|| anyhow!("AndroidKit APK path is not valid UTF-8"))?,
            ],
        )?;
        Ok(())
    }

    pub async fn start_androidkit(&self, serial: &str) -> Result<()> {
        self.run_shell(
            serial,
            "nohup am instrument -w -r xyz.smbcloud.xcrs.androidkit/.AndroidKitInstrumentation >/dev/null 2>&1 &",
        )
        .with_context(|| format!("failed to start AndroidKit on {serial}"))?;
        AndroidKit::with_adb_path(self.adb_path.clone())
            .wait_until_ready(serial)
            .await
            .with_context(|| format!("failed to start AndroidKit on {serial}"))
    }

    pub fn stop_androidkit(&self, serial: &str) -> Result<()> {
        self.run_shell(
            serial,
            &format!("am force-stop {}", AndroidKit::PACKAGE_NAME),
        )?;
        Ok(())
    }

    fn forward_androidkit(&self, serial: &str) -> Result<u16> {
        let output = run_command(
            &self.adb_path,
            [
                "-s",
                serial,
                "forward",
                "tcp:0",
                "localabstract:xcrs-androidkit",
            ],
        )?;
        output
            .trim()
            .parse::<u16>()
            .with_context(|| format!("adb returned an invalid AndroidKit forward port: {output:?}"))
    }

    fn remove_forward(&self, serial: &str, local_port: u16) -> Result<()> {
        run_command(
            &self.adb_path,
            [
                "-s",
                serial,
                "forward",
                "--remove",
                &format!("tcp:{local_port}"),
            ],
        )?;
        Ok(())
    }

    fn run_shell(&self, serial: &str, command: &str) -> Result<String> {
        run_command(&self.adb_path, ["-s", serial, "shell", command])
    }

    fn run_for_device_bytes<I, S>(&self, serial: &str, args: I) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.adb_path)
            .arg("-s")
            .arg(serial)
            .args(args)
            .output()
            .with_context(|| format!("failed to run {}", self.adb_path.display()))?;
        command_output_to_bytes(&self.adb_path, output)
    }
}

fn discover_adb_path() -> PathBuf {
    let executable = if cfg!(windows) { "adb.exe" } else { "adb" };
    let mut candidates = ["ANDROID_SDK_ROOT", "ANDROID_HOME"]
        .into_iter()
        .filter_map(env::var_os)
        .map(PathBuf::from)
        .map(|path| path.join("platform-tools").join(executable))
        .collect::<Vec<_>>();

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        candidates.push(
            home.join("Library")
                .join("Android")
                .join("sdk")
                .join("platform-tools")
                .join(executable),
        );
        candidates.push(
            home.join("Android")
                .join("Sdk")
                .join("platform-tools")
                .join(executable),
        );
    }
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        candidates.push(
            local_app_data
                .join("Android")
                .join("Sdk")
                .join("platform-tools")
                .join(executable),
        );
    }

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(executable))
}

fn parse_adb_devices(output: &str) -> Result<Vec<AndroidDevice>> {
    let mut devices = Vec::new();
    for line in output.lines().map(str::trim) {
        if line.is_empty() || line.starts_with("List of devices attached") || line.starts_with('*')
        {
            continue;
        }

        let fields = line.split_whitespace().collect::<Vec<_>>();
        let serial = fields
            .first()
            .ok_or_else(|| anyhow!("adb device row did not contain a serial: {line}"))?;
        let state = fields
            .get(1)
            .ok_or_else(|| anyhow!("adb device row did not contain a state: {line}"))?;
        let (state, property_start) = if *state == "no" && fields.get(2) == Some(&"permissions") {
            ("no permissions", 3)
        } else {
            (*state, 2)
        };
        let properties = fields[property_start..]
            .iter()
            .filter_map(|field| field.split_once(':'))
            .collect::<BTreeMap<_, _>>();

        devices.push(AndroidDevice {
            serial: (*serial).to_string(),
            state: state.to_string(),
            product: properties.get("product").map(|value| (*value).to_string()),
            model: properties.get("model").map(|value| (*value).to_string()),
            device: properties.get("device").map(|value| (*value).to_string()),
            transport_id: properties
                .get("transport_id")
                .map(|value| (*value).to_string()),
        });
    }
    Ok(devices)
}

fn validate_android_package_name(package_name: &str) -> Result<()> {
    if package_name.is_empty()
        || !package_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_'))
    {
        return Err(anyhow!(
            "Android package name must contain only ASCII letters, digits, dots, and underscores"
        ));
    }
    Ok(())
}

fn encode_adb_input_text(text: &str) -> Result<String> {
    if text.is_empty() || text.contains(['\0', '\n', '\r']) {
        return Err(anyhow!("text must be a non-empty single-line value"));
    }
    if text.contains("%s") {
        return Err(anyhow!(
            "text containing the literal sequence '%s' is not supported by adb input"
        ));
    }
    Ok(text.replace(' ', "%s"))
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// The name of the Swift helper that reads the host machine.
const HOST_INSPECT_HELPER: &str = "xcrs-host-inspect";

/// What one `xcrs-host-inspect` run wrote, whether or not it succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInspectInvocation {
    /// The helper's exit code, or `None` when a signal killed it.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl HostInspectInvocation {
    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Whether the helper rejected the invocation itself rather than failing
    /// the work it was asked to do. The helper reserves exit code 2 for a
    /// malformed command and 1 for a failed operation.
    pub fn is_usage_error(&self) -> bool {
        self.exit_code == Some(2)
    }

    /// The failure text, preferring stderr and falling back to stdout.
    pub fn message(&self) -> String {
        [self.stderr.trim(), self.stdout.trim()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Converts between the units a caller reasons about and image pixels.
///
/// A Retina screenshot is two pixels per point, so a scale of `2` accepts and
/// reports points; the default of `1` accepts and reports pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostInspectScale(f64);

impl HostInspectScale {
    /// Pixels, which is the helper's default.
    pub const PIXELS: Self = Self(1.0);

    pub fn new(pixels_per_unit: f64) -> Result<Self> {
        if !pixels_per_unit.is_finite() || pixels_per_unit <= 0.0 {
            return Err(anyhow!(
                "scale must be a positive number, got {pixels_per_unit}"
            ));
        }

        Ok(Self(pixels_per_unit))
    }

    pub fn pixels_per_unit(&self) -> f64 {
        self.0
    }

    pub fn pixels(&self, units: f64) -> f64 {
        units * self.0
    }

    pub fn units(&self, pixels: f64) -> f64 {
        pixels / self.0
    }
}

impl Default for HostInspectScale {
    fn default() -> Self {
        Self::PIXELS
    }
}

impl std::fmt::Display for HostInspectScale {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::str::FromStr for HostInspectScale {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let pixels_per_unit = value
            .trim()
            .parse::<f64>()
            .with_context(|| format!("scale expects a number, got {value}"))?;

        Self::new(pixels_per_unit)
    }
}

/// A rectangle in caller units. Rows run top down and columns left to right,
/// matching the rest of XCRS.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HostInspectRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl HostInspectRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if [self.x, self.y, self.width, self.height]
            .into_iter()
            .any(|value| !value.is_finite())
        {
            return Err(anyhow!("rect values must be finite numbers, got {self}"));
        }
        if self.width <= 0.0 || self.height <= 0.0 {
            return Err(anyhow!(
                "rect width and height must be positive, got {self}"
            ));
        }

        Ok(())
    }
}

impl std::fmt::Display for HostInspectRect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{},{},{},{}",
            self.x, self.y, self.width, self.height
        )
    }
}

impl std::str::FromStr for HostInspectRect {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let parts = value.split(',').collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(anyhow!("rect expects x,y,w,h, got {value}"));
        }

        let mut numbers = [0.0_f64; 4];
        for (index, part) in parts.into_iter().enumerate() {
            numbers[index] = part
                .trim()
                .parse::<f64>()
                .with_context(|| format!("rect values must be numbers, got {value}"))?;
        }

        let rect = Self::new(numbers[0], numbers[1], numbers[2], numbers[3]);
        rect.validate()?;

        Ok(rect)
    }
}

/// The image a result was read from, in both pixels and caller units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInspectImage {
    pub path: String,
    pub pixel_width: i64,
    pub pixel_height: i64,
    pub scale: f64,
    pub width: f64,
    pub height: f64,
}

/// An on-screen window in the global display space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostWindow {
    pub id: i64,
    pub owner: String,
    pub title: String,
    pub layer: i64,
    /// Screen points, unaffected by the requested scale.
    pub bounds: HostInspectRect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostWindowList {
    pub windows: Vec<HostWindow>,
}

/// One recognized text box, ordered top to bottom and then left to right.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostRecognizedText {
    pub text: String,
    pub confidence: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostOcrResult {
    pub image: HostInspectImage,
    /// The languages the request was configured with; empty means the
    /// platform's own default.
    pub languages: Vec<String>,
    pub results: Vec<HostRecognizedText>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCropResult {
    pub image: HostInspectImage,
    /// The requested rect, in caller units.
    pub rect: HostInspectRect,
    pub out: String,
    pub pixel_width: i64,
    pub pixel_height: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostProbeAxis {
    Row,
    Column,
}

impl HostProbeAxis {
    /// The flag the helper reads this axis from.
    pub fn flag(&self) -> &'static str {
        match self {
            Self::Row => "--row",
            Self::Column => "--column",
        }
    }
}

impl std::fmt::Display for HostProbeAxis {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Row => "row",
            Self::Column => "column",
        })
    }
}

/// A stretch of one color along a probed line, half open in caller units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostColorRun {
    pub start: f64,
    pub end: f64,
    pub width: f64,
    /// Quantized `#rrggbb`, so low-order noise does not split a surface.
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostProbeResult {
    pub image: HostInspectImage,
    pub axis: HostProbeAxis,
    pub position: f64,
    pub from: f64,
    pub to: f64,
    pub runs: Vec<HostColorRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostOcrRequest {
    pub image: PathBuf,
    pub scale: HostInspectScale,
    /// Empty leaves the platform on its own default.
    pub languages: Vec<String>,
}

impl HostOcrRequest {
    pub fn new(image: impl Into<PathBuf>) -> Self {
        Self {
            image: image.into(),
            scale: HostInspectScale::PIXELS,
            languages: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostCropRequest {
    pub image: PathBuf,
    pub rect: HostInspectRect,
    pub out: PathBuf,
    pub scale: HostInspectScale,
}

impl HostCropRequest {
    pub fn new(image: impl Into<PathBuf>, rect: HostInspectRect, out: impl Into<PathBuf>) -> Self {
        Self {
            image: image.into(),
            rect,
            out: out.into(),
            scale: HostInspectScale::PIXELS,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostProbeRequest {
    pub image: PathBuf,
    pub axis: HostProbeAxis,
    pub position: f64,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub scale: HostInspectScale,
}

impl HostProbeRequest {
    pub fn new(image: impl Into<PathBuf>, axis: HostProbeAxis, position: f64) -> Self {
        Self {
            image: image.into(),
            axis,
            position,
            from: None,
            to: None,
            scale: HostInspectScale::PIXELS,
        }
    }
}

/// Client for the `xcrs-host-inspect` helper, which reads the macOS host on
/// behalf of XCRS control runs: on-screen windows, image pixels, and text.
#[derive(Debug, Clone)]
pub struct HostInspect {
    helper_path: PathBuf,
}

#[cfg(unix)]
fn is_text_file_busy(result: &std::io::Result<Output>) -> bool {
    matches!(result, Err(error) if error.raw_os_error() == Some(TEXT_FILE_BUSY))
}

#[cfg(not(unix))]
fn is_text_file_busy(_result: &std::io::Result<Output>) -> bool {
    false
}

impl Default for HostInspect {
    fn default() -> Self {
        Self {
            helper_path: discover_host_inspect_path(),
        }
    }
}

impl HostInspect {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_path(helper_path: impl Into<PathBuf>) -> Self {
        Self {
            helper_path: helper_path.into(),
        }
    }

    pub fn helper_path(&self) -> &Path {
        &self.helper_path
    }

    /// Run the helper and report what it wrote.
    ///
    /// A non-zero exit is part of the helper's contract rather than an error
    /// here, so a caller can forward the code instead of re-deriving it. This
    /// only fails when the helper could not be started at all.
    pub fn invoke(&self, arguments: &[String]) -> Result<HostInspectInvocation> {
        let output = self.run_helper(arguments).with_context(|| {
            format!(
                "failed to run {} (build it with `swift build` in xcrs-controlkit)",
                self.helper_path.display()
            )
        })?;

        Ok(HostInspectInvocation {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    /// Start the helper, waiting out a file that is briefly unstartable.
    ///
    /// Unix refuses to `exec` an executable that any process still holds open
    /// for writing, and reports that as `ETXTBSY` rather than as a missing or
    /// broken binary. A `swift build` replacing the helper does it, and so does
    /// a sibling thread forking while a test writes a stand-in; both last only
    /// until the other side closes its descriptor, so a short retry is the
    /// difference between a transient condition and a reported failure.
    fn run_helper(&self, arguments: &[String]) -> std::io::Result<Output> {
        let mut attempt = 0;
        loop {
            let started = Command::new(&self.helper_path).args(arguments).output();
            if !is_text_file_busy(&started) || attempt >= TEXT_FILE_BUSY_RETRY_ATTEMPTS {
                return started;
            }

            attempt += 1;
            std::thread::sleep(TEXT_FILE_BUSY_RETRY_DELAY);
        }
    }

    /// List the host's on-screen windows, ordered by layer and then id.
    pub fn windows(&self, owner: Option<&str>) -> Result<HostWindowList> {
        let mut arguments = vec!["windows".to_string()];
        if let Some(owner) = owner {
            arguments.push("--owner".to_string());
            arguments.push(owner.to_string());
        }

        self.decode(&arguments)
    }

    /// Read the text in an image.
    pub fn recognize_text(&self, request: &HostOcrRequest) -> Result<HostOcrResult> {
        let mut arguments = vec![
            "ocr".to_string(),
            "--image".to_string(),
            path_to_string(&request.image)?,
            "--scale".to_string(),
            request.scale.to_string(),
        ];
        for language in &request.languages {
            arguments.push("--language".to_string());
            arguments.push(language.clone());
        }

        self.decode(&arguments)
    }

    /// Crop an image to a rect and write the result out as a PNG.
    pub fn crop(&self, request: &HostCropRequest) -> Result<HostCropResult> {
        request.rect.validate()?;
        let arguments = vec![
            "crop".to_string(),
            "--image".to_string(),
            path_to_string(&request.image)?,
            "--rect".to_string(),
            request.rect.to_string(),
            "--out".to_string(),
            path_to_string(&request.out)?,
            "--scale".to_string(),
            request.scale.to_string(),
        ];

        self.decode(&arguments)
    }

    /// Read one line of an image as a run of colors.
    pub fn probe(&self, request: &HostProbeRequest) -> Result<HostProbeResult> {
        if !request.position.is_finite() {
            return Err(anyhow!(
                "probe position must be a finite number, got {}",
                request.position
            ));
        }

        let mut arguments = vec![
            "probe".to_string(),
            "--image".to_string(),
            path_to_string(&request.image)?,
            request.axis.flag().to_string(),
            request.position.to_string(),
            "--scale".to_string(),
            request.scale.to_string(),
        ];
        for (flag, bound) in [("--from", request.from), ("--to", request.to)] {
            let Some(bound) = bound else { continue };
            if !bound.is_finite() {
                return Err(anyhow!("probe {flag} must be a finite number, got {bound}"));
            }
            arguments.push(flag.to_string());
            arguments.push(bound.to_string());
        }

        self.decode(&arguments)
    }

    fn decode<T>(&self, arguments: &[String]) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let command = arguments
            .first()
            .map(String::as_str)
            .unwrap_or("invocation");
        let invocation = self.invoke(arguments)?;
        if !invocation.succeeded() {
            let status = match invocation.exit_code {
                Some(code) => format!("exited with status {code}"),
                None => "was killed by a signal".to_string(),
            };
            let message = invocation.message();

            return Err(if message.is_empty() {
                anyhow!("xcrs-host-inspect {command} {status}")
            } else {
                anyhow!("xcrs-host-inspect {command} {status}: {message}")
            });
        }

        serde_json::from_str(&invocation.stdout)
            .with_context(|| format!("failed to decode xcrs-host-inspect {command} JSON"))
    }
}

fn discover_host_inspect_path() -> PathBuf {
    let executable = if cfg!(windows) {
        format!("{HOST_INSPECT_HELPER}.exe")
    } else {
        HOST_INSPECT_HELPER.to_string()
    };

    let mut candidates = Vec::new();
    if let Some(path) = env::var_os("XCRS_HOST_INSPECT") {
        candidates.push(PathBuf::from(path));
    }
    // A release ships the helper next to the `xcrs` binary; a SwiftPM checkout
    // leaves it in the build directory.
    if let Some(directory) = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        candidates.push(directory.join(&executable));
    }
    candidates.push(Path::new(".build").join("release").join(&executable));
    candidates.push(Path::new(".build").join("debug").join(&executable));
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        candidates.push(home.join("bin").join(&executable));
    }
    candidates.push(Path::new("/usr/local/bin").join(&executable));
    candidates.push(Path::new("/opt/homebrew/bin").join(&executable));

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(executable))
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

fn command_output_to_bytes(program: &Path, output: Output) -> Result<Vec<u8>> {
    if output.status.success() {
        return Ok(output.stdout);
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

fn parse_resolved_android_activity(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .rev()
        .find(|line| !line.is_empty() && line.contains('/') && !line.contains(char::is_whitespace))
        .map(str::to_string)
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
    fn reports_actionable_controlkit_method_mismatch() {
        let error = serde_json::json!({
            "code": -32601,
            "message": "Method not found"
        });
        let runner_info = serde_json::json!({
            "runner": "XCRSControlKit",
            "protocolVersion": 1
        });

        let message = controlkit_rpc_error("device.dump.ui", &error, Some(&runner_info), None);

        assert!(message.contains("XCRSControlKit protocol 1"));
        assert!(message.contains("device.dump.ui"));
        assert!(message.contains("Rebuild or upgrade"));
    }

    #[test]
    fn recognizes_integral_float_controlkit_method_not_found_code() {
        let error = serde_json::json!({
            "code": -32601.0,
            "message": "Method not found"
        });

        assert!(is_controlkit_method_not_found(&error));
        let message = controlkit_rpc_error("device.dump.ui", &error, None, None);
        assert!(message.contains("does not implement ControlKit method 'device.dump.ui'"));
    }

    #[test]
    fn reports_when_controlkit_runner_metadata_is_unavailable() {
        let error = serde_json::json!({
            "code": -32601,
            "message": "Method not found"
        });

        let message = controlkit_rpc_error(
            "device.dump.ui",
            &error,
            None,
            Some("device.info returned a JSON-RPC error"),
        );

        assert!(message.contains("The connected ControlKit runner"));
        assert!(message.contains("Runner metadata could not be read"));
        assert!(message.contains("device.info returned a JSON-RPC error"));
        assert!(!message.contains("unknown ControlKit runner"));
    }

    #[test]
    fn extracts_identified_visible_controlkit_elements() {
        let hierarchy = serde_json::json!({
            "type": "Application",
            "rect": { "x": 0, "y": 0, "width": 1920, "height": 1080 },
            "children": [
                {
                    "type": "Button",
                    "label": "Play",
                    "rawIdentifier": "play-button",
                    "rect": { "x": 100, "y": 200, "width": 80, "height": 40 },
                    "enabled": true,
                    "selected": false,
                    "hittable": true,
                    "children": []
                },
                {
                    "type": "Image",
                    "label": "",
                    "rect": { "x": 0, "y": 0, "width": 40, "height": 40 },
                    "children": []
                }
            ]
        });

        let elements = extract_controlkit_elements(&hierarchy);

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0]["label"], serde_json::json!("Play"));
        assert_eq!(elements[0]["hittable"], serde_json::json!(true));
    }

    #[test]
    fn normalizes_visible_androidkit_elements() {
        let hierarchy = serde_json::json!({
            "hierarchy": [{
                "role": "android.widget.Button",
                "text": "Continue",
                "hint": "",
                "contentDescription": "",
                "resourceId": "com.example:id/continue",
                "enabled": true,
                "clickable": true,
                "selected": false,
                "visible": true,
                "rect": { "x": 10, "y": 20, "width": 100, "height": 40 }
            }]
        });

        let elements = extract_androidkit_elements(&hierarchy);

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0]["label"], serde_json::json!("Continue"));
        assert_eq!(
            elements[0]["rawIdentifier"],
            serde_json::json!("com.example:id/continue")
        );
    }

    #[test]
    fn parses_device_tunnel_address() {
        let output = "• Device Name: iPhone\n• Tunnel IP Address: fd55:33ce:ad87::1\n";
        assert_eq!(
            parse_device_tunnel_address(output).as_deref(),
            Some("fd55:33ce:ad87::1")
        );
    }

    #[test]
    fn parses_adb_devices() {
        let output = r#"List of devices attached
emulator-5554          device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 device:emu64a transport_id:1
SERIAL123              unauthorized usb:0-1 transport_id:2
SERIAL456              no permissions (user denied) usb:0-2 transport_id:3
"#;

        let devices = parse_adb_devices(output).expect("devices should parse");

        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].serial, "emulator-5554");
        assert_eq!(devices[0].state, "device");
        assert_eq!(devices[0].model.as_deref(), Some("sdk_gphone64_arm64"));
        assert_eq!(devices[1].serial, "SERIAL123");
        assert_eq!(devices[1].state, "unauthorized");
        assert_eq!(devices[2].serial, "SERIAL456");
        assert_eq!(devices[2].state, "no permissions");
    }

    #[test]
    fn encodes_adb_input_spaces() {
        let text = encode_adb_input_text("hello android").expect("text should encode");
        assert_eq!(text, "hello%sandroid");
    }

    #[test]
    fn rejects_literal_adb_space_escape() {
        let error = encode_adb_input_text("literal%svalue").expect_err("text should be rejected");
        assert!(error.to_string().contains("literal sequence"));
    }

    #[test]
    fn parses_resolved_android_activity() {
        let output = "priority=0 preferredOrder=0 match=0x108000\n\
                      ai.splitfire.KaroKowe/.MainActivity\n";

        assert_eq!(
            parse_resolved_android_activity(output).as_deref(),
            Some("ai.splitfire.KaroKowe/.MainActivity")
        );
    }

    #[test]
    fn escapes_android_shell_arguments() {
        assert_eq!(shell_escape("don't"), "'don'\\''t'");
    }

    #[test]
    fn normalizes_apple_simulator_as_simulator_kind() {
        let simulator = Simulator {
            platform: ApplePlatform::Ios,
            runtime_identifier: "com.apple.CoreSimulator.SimRuntime.iOS-26-0".to_string(),
            name: "Test-iOS-26".to_string(),
            udid: "00000000-0000-0000-0000-000000000000".to_string(),
            state: "Booted".to_string(),
            is_available: true,
        };

        let normalized = NormalizedDevice::from_simulator(&simulator);

        assert_eq!(normalized.platform, DevicePlatform::Apple);
        assert_eq!(normalized.kind, DeviceKind::Simulator);
        assert_eq!(normalized.identifier, simulator.udid);
        assert_eq!(normalized.name, "Test-iOS-26");
        assert_eq!(normalized.state, "Booted");
        assert_eq!(normalized.transport, "simctl");
    }

    #[test]
    fn normalizes_android_emulator_serial_as_emulator_kind() {
        let device = AndroidDevice {
            serial: "emulator-5554".to_string(),
            state: "device".to_string(),
            product: Some("sdk_gphone64_arm64".to_string()),
            model: Some("sdk_gphone64_arm64".to_string()),
            device: Some("emu64a".to_string()),
            transport_id: Some("1".to_string()),
        };

        let normalized = NormalizedDevice::from_android_device(&device);

        assert_eq!(normalized.platform, DevicePlatform::Android);
        assert_eq!(normalized.kind, DeviceKind::Emulator);
        assert_eq!(normalized.identifier, "emulator-5554");
        assert_eq!(normalized.name, "sdk_gphone64_arm64");
        assert_eq!(normalized.transport, "adb");
    }

    #[test]
    fn normalizes_android_physical_serial_as_physical_device_kind() {
        let device = AndroidDevice {
            serial: "R58N123ABCD".to_string(),
            state: "device".to_string(),
            product: None,
            model: None,
            device: None,
            transport_id: None,
        };

        let normalized = NormalizedDevice::from_android_device(&device);

        assert_eq!(normalized.kind, DeviceKind::PhysicalDevice);
        // Falls back to the serial when no product/model/device property is reported.
        assert_eq!(normalized.name, "R58N123ABCD");
    }

    #[test]
    fn defaults_host_inspect_scale_to_pixels() {
        let scale = HostInspectScale::default();

        assert_eq!(scale, HostInspectScale::PIXELS);
        assert_eq!(scale.pixels_per_unit(), 1.0);
        assert_eq!(scale.to_string(), "1");
    }

    #[test]
    fn converts_between_units_and_pixels() {
        let retina = HostInspectScale::new(2.0).expect("2 is a valid scale");

        assert_eq!(retina.pixels(720.0), 1440.0);
        assert_eq!(retina.units(1440.0), 720.0);
        assert_eq!(retina.to_string(), "2");
    }

    #[test]
    fn parses_host_inspect_scale() {
        let scale = "2.5".parse::<HostInspectScale>().expect("2.5 should parse");

        assert_eq!(scale.pixels_per_unit(), 2.5);
        assert!("0".parse::<HostInspectScale>().is_err());
        assert!("-1".parse::<HostInspectScale>().is_err());
        assert!("nan".parse::<HostInspectScale>().is_err());
        assert!("two".parse::<HostInspectScale>().is_err());
    }

    #[test]
    fn parses_and_formats_host_inspect_rect() {
        let rect = "10, 20, 30, 40"
            .parse::<HostInspectRect>()
            .expect("a four number rect should parse");

        assert_eq!(rect, HostInspectRect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(rect.to_string(), "10,20,30,40");
        assert_eq!(
            rect.to_string()
                .parse::<HostInspectRect>()
                .expect("round trip"),
            rect
        );
    }

    #[test]
    fn rejects_a_rect_that_cannot_be_used() {
        assert!("0,0,4".parse::<HostInspectRect>().is_err());
        assert!("0,0,4,4,4".parse::<HostInspectRect>().is_err());
        assert!("a,b,c,d".parse::<HostInspectRect>().is_err());
        assert!("0,0,0,4".parse::<HostInspectRect>().is_err());
        assert!("0,0,4,-1".parse::<HostInspectRect>().is_err());
        assert!("0,0,inf,4".parse::<HostInspectRect>().is_err());
    }

    #[test]
    fn discovers_the_host_inspect_helper_from_the_environment() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let helper = directory.path().join("inspect-from-environment");
        std::fs::write(&helper, "#!/bin/sh\n").expect("the helper should write");

        let previous = env::var_os("XCRS_HOST_INSPECT");
        env::set_var("XCRS_HOST_INSPECT", &helper);
        let discovered = discover_host_inspect_path();
        match previous {
            Some(value) => env::set_var("XCRS_HOST_INSPECT", value),
            None => env::remove_var("XCRS_HOST_INSPECT"),
        }

        // The environment wins over every other candidate, exactly as it does
        // for the Android debug bridge.
        assert_eq!(discovered, helper);
    }

    /// Writes a stand-in for the Swift helper so the client can be exercised
    /// without a built `xcrs-host-inspect` or a macOS host.
    #[cfg(unix)]
    fn write_fake_helper(directory: &Path, script: &str) -> PathBuf {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let path = directory.join(HOST_INSPECT_HELPER);
        // One handle writes the script and carries the executable bit, so the
        // file is never re-opened and its descriptor is closed before the test
        // starts it.
        let mut helper = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o755)
            .open(&path)
            .expect("the fake helper should open");
        helper
            .write_all(script.as_bytes())
            .expect("the fake helper should write");
        drop(helper);

        path
    }

    /// A fake helper that records the arguments it was handed and prints
    /// `stdout` before exiting with `exit_code`.
    #[cfg(unix)]
    fn recording_helper(directory: &Path, stdout: &str, exit_code: i32) -> (PathBuf, PathBuf) {
        let recorded = directory.join("arguments.txt");
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\ncat <<'EOF'\n{}\nEOF\nexit {}\n",
            recorded.display(),
            stdout,
            exit_code
        );

        (write_fake_helper(directory, &script), recorded)
    }

    #[cfg(unix)]
    fn recorded_arguments(recorded: &Path) -> Vec<String> {
        std::fs::read_to_string(recorded)
            .expect("the helper should have recorded its arguments")
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[cfg(unix)]
    const WINDOW_LIST_JSON: &str = r#"{
      "windows": [
        {
          "id": 42,
          "owner": "Safari",
          "title": "Example",
          "layer": 0,
          "bounds": { "x": 0, "y": 25, "width": 1440, "height": 875 }
        }
      ]
    }"#;

    #[cfg(unix)]
    const IMAGE_JSON: &str = r#"{
      "path": "/tmp/screenshot.png",
      "pixelWidth": 2880,
      "pixelHeight": 1800,
      "scale": 2,
      "width": 1440,
      "height": 900
    }"#;

    #[cfg(unix)]
    #[test]
    fn decodes_a_host_window_list() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let (helper, _) = recording_helper(directory.path(), WINDOW_LIST_JSON, 0);
        let host = HostInspect::with_path(helper);

        let list = host.windows(None).expect("the window list should decode");

        assert_eq!(list.windows.len(), 1);
        let window = &list.windows[0];
        assert_eq!(window.id, 42);
        assert_eq!(window.owner, "Safari");
        assert_eq!(window.title, "Example");
        assert_eq!(window.layer, 0);
        // Bounds stay in screen points, so they are not scaled.
        assert_eq!(
            window.bounds,
            HostInspectRect::new(0.0, 25.0, 1440.0, 875.0)
        );
    }

    #[cfg(unix)]
    #[test]
    fn waits_out_a_helper_that_is_still_open_for_writing() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let (helper, recorded) = recording_helper(directory.path(), WINDOW_LIST_JSON, 0);
        let host = HostInspect::with_path(&helper);

        // Unix refuses to start a file that is open for writing anywhere, and
        // that is what a sibling thread forking mid-write produces. Holding the
        // descriptor here reproduces it on demand.
        let held = std::fs::OpenOptions::new()
            .write(true)
            .open(&helper)
            .expect("the helper should reopen for writing");
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            drop(held);
        });

        host.windows(None)
            .expect("windows should wait out the open handle");
        release.join().expect("the releasing thread should finish");

        assert_eq!(recorded_arguments(&recorded), vec!["windows"]);
    }

    #[cfg(unix)]
    #[test]
    fn builds_the_windows_arguments() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let (helper, recorded) = recording_helper(directory.path(), WINDOW_LIST_JSON, 0);
        let host = HostInspect::with_path(helper);

        host.windows(None).expect("windows should succeed");
        assert_eq!(recorded_arguments(&recorded), vec!["windows"]);

        host.windows(Some("Safari"))
            .expect("a filtered windows call should succeed");
        assert_eq!(
            recorded_arguments(&recorded),
            vec!["windows", "--owner", "Safari"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn builds_the_ocr_arguments_and_decodes_the_result() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let stdout = format!(
            r#"{{
              "image": {},
              "languages": ["en-US", "sv-SE"],
              "results": [
                {{
                  "text": "Sign in",
                  "confidence": 0.9,
                  "x": 10,
                  "y": 20,
                  "width": 100,
                  "height": 30
                }}
              ]
            }}"#,
            IMAGE_JSON
        );
        let (helper, recorded) = recording_helper(directory.path(), &stdout, 0);
        let host = HostInspect::with_path(helper);

        let mut request = HostOcrRequest::new("/tmp/screenshot.png");
        request.scale = HostInspectScale::new(2.0).expect("2 is a valid scale");
        request.languages = vec!["en-US".to_string(), "sv-SE".to_string()];

        let result = host
            .recognize_text(&request)
            .expect("the OCR result should decode");

        assert_eq!(
            recorded_arguments(&recorded),
            vec![
                "ocr",
                "--image",
                "/tmp/screenshot.png",
                "--scale",
                "2",
                "--language",
                "en-US",
                "--language",
                "sv-SE",
            ]
        );
        // Image geometry is reported in caller units beside the pixel size.
        assert_eq!(result.image.pixel_width, 2880);
        assert_eq!(result.image.width, 1440.0);
        assert_eq!(result.image.scale, 2.0);
        assert_eq!(result.languages, vec!["en-US", "sv-SE"]);
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].text, "Sign in");
        assert_eq!(result.results[0].confidence, 0.9);
    }

    #[cfg(unix)]
    #[test]
    fn builds_the_crop_arguments_and_decodes_the_result() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let stdout = format!(
            r#"{{
              "image": {},
              "rect": {{ "x": 0, "y": 0, "width": 4, "height": 4 }},
              "out": "/tmp/crop.png",
              "pixelWidth": 8,
              "pixelHeight": 8
            }}"#,
            IMAGE_JSON
        );
        let (helper, recorded) = recording_helper(directory.path(), &stdout, 0);
        let host = HostInspect::with_path(helper);
        let request = HostCropRequest::new(
            "/tmp/screenshot.png",
            HostInspectRect::new(0.0, 0.0, 4.0, 4.0),
            "/tmp/crop.png",
        );

        let result = host.crop(&request).expect("the crop result should decode");

        assert_eq!(
            recorded_arguments(&recorded),
            vec![
                "crop",
                "--image",
                "/tmp/screenshot.png",
                "--rect",
                "0,0,4,4",
                "--out",
                "/tmp/crop.png",
                "--scale",
                "1",
            ]
        );
        assert_eq!(result.rect, HostInspectRect::new(0.0, 0.0, 4.0, 4.0));
        assert_eq!(result.out, "/tmp/crop.png");
        assert_eq!(result.pixel_width, 8);
        assert_eq!(result.pixel_height, 8);
    }

    #[cfg(unix)]
    #[test]
    fn builds_the_probe_arguments_and_decodes_the_result() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let stdout = format!(
            r##"{{
              "image": {},
              "axis": "column",
              "position": 1,
              "from": 4,
              "to": 8,
              "runs": [
                {{ "start": 4, "end": 8, "width": 4, "color": "#0000f8" }}
              ]
            }}"##,
            IMAGE_JSON
        );
        let (helper, recorded) = recording_helper(directory.path(), &stdout, 0);
        let host = HostInspect::with_path(helper);

        let mut request = HostProbeRequest::new("/tmp/screenshot.png", HostProbeAxis::Column, 1.0);
        request.from = Some(4.0);
        request.to = Some(8.0);

        let result = host
            .probe(&request)
            .expect("the probe result should decode");

        assert_eq!(
            recorded_arguments(&recorded),
            vec![
                "probe",
                "--image",
                "/tmp/screenshot.png",
                "--column",
                "1",
                "--scale",
                "1",
                "--from",
                "4",
                "--to",
                "8",
            ]
        );
        assert_eq!(result.axis, HostProbeAxis::Column);
        assert_eq!(result.position, 1.0);
        assert_eq!(result.from, 4.0);
        assert_eq!(result.to, 8.0);
        assert_eq!(result.runs.len(), 1);
        assert_eq!(result.runs[0].color, "#0000f8");
        assert_eq!(result.runs[0].start, 4.0);
        assert_eq!(result.runs[0].end, 8.0);
    }

    #[cfg(unix)]
    #[test]
    fn omits_the_unset_probe_bounds() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let stdout = format!(
            r#"{{ "image": {}, "axis": "row", "position": 1, "from": 0, "to": 8, "runs": [] }}"#,
            IMAGE_JSON
        );
        let (helper, recorded) = recording_helper(directory.path(), &stdout, 0);
        let host = HostInspect::with_path(helper);
        let request = HostProbeRequest::new("/tmp/screenshot.png", HostProbeAxis::Row, 1.0);

        host.probe(&request).expect("the probe should succeed");

        assert_eq!(
            recorded_arguments(&recorded),
            vec![
                "probe",
                "--image",
                "/tmp/screenshot.png",
                "--row",
                "1",
                "--scale",
                "1",
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn keeps_the_host_inspect_exit_code_as_data() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let (helper, _) = recording_helper(directory.path(), "", 2);
        let host = HostInspect::with_path(helper);

        let invocation = host
            .invoke(&["windows".to_string()])
            .expect("a usage error is still a completed run");

        assert_eq!(invocation.exit_code, Some(2));
        assert!(invocation.is_usage_error());
        assert!(!invocation.succeeded());
    }

    #[cfg(unix)]
    #[test]
    fn reports_a_failed_helper_with_its_message() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let script =
            "#!/bin/sh\necho 'cannot open an image /nonexistent/missing.png' >&2\nexit 1\n";
        let helper = write_fake_helper(directory.path(), script);
        let host = HostInspect::with_path(helper);

        let error = host
            .windows(None)
            .expect_err("a failed helper should not decode");

        let message = error.to_string();
        assert!(message.contains("xcrs-host-inspect windows exited with status 1"));
        assert!(message.contains("cannot open an image /nonexistent/missing.png"));
    }

    #[cfg(unix)]
    #[test]
    fn reports_a_silently_failed_helper_without_a_dangling_message() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let (helper, _) = recording_helper(directory.path(), "", 1);
        let host = HostInspect::with_path(helper);

        let error = host
            .windows(None)
            .expect_err("a failed helper should not decode");

        assert_eq!(
            error.to_string(),
            "xcrs-host-inspect windows exited with status 1"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_output_that_is_not_json() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let (helper, _) = recording_helper(directory.path(), "not json", 0);
        let host = HostInspect::with_path(helper);

        let error = host
            .windows(None)
            .expect_err("malformed output should not decode");

        assert!(error
            .to_string()
            .contains("failed to decode xcrs-host-inspect windows JSON"));
    }

    #[test]
    fn reports_a_missing_host_inspect_helper() {
        let host = HostInspect::with_path("/nonexistent/xcrs-host-inspect");

        let error = host
            .windows(None)
            .expect_err("a missing helper should not run");

        let message = error.to_string();
        assert!(message.contains("failed to run /nonexistent/xcrs-host-inspect"));
        assert!(message.contains("swift build"));
    }

    #[test]
    fn validates_before_running_the_helper() {
        // Point at a helper that cannot exist, so any error other than the
        // validation one would prove the request was sent anyway.
        let host = HostInspect::with_path("/nonexistent/xcrs-host-inspect");

        let error = host
            .crop(&HostCropRequest::new(
                "/tmp/screenshot.png",
                HostInspectRect::new(0.0, 0.0, 0.0, 4.0),
                "/tmp/crop.png",
            ))
            .expect_err("a zero width rect should be rejected");
        assert!(error.to_string().contains("must be positive"));

        let error = host
            .probe(&HostProbeRequest::new(
                "/tmp/screenshot.png",
                HostProbeAxis::Row,
                f64::NAN,
            ))
            .expect_err("a non-finite position should be rejected");
        assert!(error.to_string().contains("must be a finite number"));
    }
}
