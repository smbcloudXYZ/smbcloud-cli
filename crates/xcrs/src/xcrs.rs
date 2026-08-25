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
            let (runner_info, runner_info_error) =
                if error.get("code").and_then(serde_json::Value::as_i64)
                    == Some(CONTROLKIT_METHOD_NOT_FOUND)
                    && method != "device.info"
                {
                    match self.send("device.info", serde_json::json!({})).await {
                        Ok(body) => match body.get("result") {
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
                        Err(error) => (None, Some(error.to_string())),
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

    if code != CONTROLKIT_METHOD_NOT_FOUND {
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
}
