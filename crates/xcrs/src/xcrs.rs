use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub mod mcp;

const IOS_SIMULATOR_DESTINATION_PREFIX: &str = "platform=iOS Simulator,id=";

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
pub struct Simulator {
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
}

pub fn parse_simctl_devices(output: &str) -> Result<Vec<Simulator>> {
    let list: SimctlDeviceList =
        serde_json::from_str(output).context("failed to parse simctl device list JSON")?;

    let mut simulators = Vec::new();
    for (runtime_identifier, devices) in list.devices {
        for device in devices {
            simulators.push(Simulator {
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
        assert_eq!(devices[0].name, "Test-iOS-26");
        assert_eq!(devices[0].state, "Booted");
        assert!(devices[0].is_booted());
    }
}
