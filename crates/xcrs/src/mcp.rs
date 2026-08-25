//! Cross-platform automation MCP contract.
//!
//! This module defines the canonical tool set exposed by `xcrs` (and re-exposed by
//! embedders such as `smb --mcp --scope automation`): `device_list`, `device_select`,
//! `device_capabilities`, `app_install_launch`, `screen_capture`, `app_launch`,
//! `app_terminate`, `url_open`, `ui_describe`, `ui_element_list`, `input_tap`,
//! `input_text`, `input_swipe`, `input_button`, `input_click`, `input_spatial_tap`,
//! `orientation_get`, and `orientation_set`. Each name is supplied by the embedder
//! through [`xcrs_mcp_tools`] so the standalone crate and CLI profile share one
//! implementation and one public contract.
//!
//! Targets are Apple simulators, Apple ControlKit hosts (physical/remote devices or
//! a local macOS runner), or Android devices reachable through adb. A target can be
//! selected once with `device_select` and reused by later calls, or passed explicitly
//! on any call. [`resolve_selected_target`] is the single, unit-tested place that
//! validates a target is unambiguous.
use {
    anyhow::{anyhow, Result},
    rmcp::{
        model::{Implementation, ServerCapabilities, ServerInfo},
        transport::stdio,
        ServerHandler, ServiceExt,
    },
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

/// A validated, unambiguous automation target: exactly one of an Apple simulator, an
/// Apple ControlKit host (physical device, remote runner, or local Mac), or an
/// Android device. Produced by [`resolve_selected_target`], never constructed
/// directly from untrusted input.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "platform", rename_all = "snake_case")]
pub enum SelectedTarget {
    /// An Apple simulator known to `simctl`, identified by name and/or UDID.
    AppleSimulator {
        #[serde(default)]
        simulator_name: Option<String>,
        #[serde(default)]
        simulator_udid: Option<String>,
        /// Local ControlKit JSON-RPC port. Defaults to 12004.
        #[serde(default)]
        controlkit_port: Option<u16>,
    },
    /// A ControlKit JSON-RPC endpoint reachable over the network: a paired physical
    /// device, a remote runner, or a local macOS app.
    AppleHost {
        host: String,
        /// Local ControlKit JSON-RPC port. Defaults to 12004.
        #[serde(default)]
        controlkit_port: Option<u16>,
    },
    /// An Android device or emulator reachable through adb.
    Android {
        /// Device serial from `adb devices -l`. `None` means "the single
        /// authorized device", resolved lazily when the target is used.
        #[serde(default)]
        serial: Option<String>,
    },
}

/// Validate and tag a target from optional per-call fields, falling back to
/// `stored` (the target previously selected with `device_select`) when none of the
/// fields are provided. Returns a plain error message (rather than an MCP error
/// type) so this function stays easy to unit test; callers wrap it for their
/// transport.
///
/// Rejects ambiguous combinations: `android_serial` together with any Apple field,
/// or `simulator_name`/`simulator_udid` together with `host`.
pub fn resolve_selected_target(
    simulator_name: Option<String>,
    simulator_udid: Option<String>,
    host: Option<String>,
    controlkit_port: Option<u16>,
    android_serial: Option<String>,
    stored: Option<SelectedTarget>,
) -> Result<SelectedTarget, String> {
    let apple_simulator_provided = simulator_name.is_some() || simulator_udid.is_some();
    let apple_host_provided = host.is_some();
    let android_provided = android_serial.is_some();

    if android_provided && (apple_simulator_provided || apple_host_provided) {
        return Err(
            "Ambiguous target: android_serial cannot be combined with simulator_name, \
             simulator_udid, or host. Provide exactly one target."
                .to_string(),
        );
    }
    if apple_simulator_provided && apple_host_provided {
        return Err(
            "Ambiguous target: simulator_name/simulator_udid cannot be combined with host. \
             Provide exactly one target."
                .to_string(),
        );
    }

    if let Some(serial) = android_serial {
        return Ok(SelectedTarget::Android {
            serial: Some(serial),
        });
    }
    if let Some(host) = host {
        return Ok(SelectedTarget::AppleHost {
            host,
            controlkit_port,
        });
    }
    if apple_simulator_provided {
        return Ok(SelectedTarget::AppleSimulator {
            simulator_name,
            simulator_udid,
            controlkit_port,
        });
    }

    stored.ok_or_else(|| {
        "No target provided. Pass simulator_name/simulator_udid, host, or android_serial, \
         or select a default target first with device_select."
            .to_string()
    })
}

/// Reject a resolved target that is Android for a tool that only works on Apple
/// platforms (UI introspection, orientation, macOS click, visionOS spatial tap).
/// Returns a message naming the offending tool so the error is actionable.
pub fn require_apple_target(tool_name: &str, target: &SelectedTarget) -> Result<(), String> {
    match target {
        SelectedTarget::Android { serial } => Err(format!(
            "{tool_name} is Apple-only; the active target is an Android device ({}). \
             Select an Apple simulator or ControlKit host with device_select, or pass \
             simulator_name, simulator_udid, or host explicitly.",
            serial.as_deref().unwrap_or("auto-detected")
        )),
        SelectedTarget::AppleSimulator { .. } | SelectedTarget::AppleHost { .. } => Ok(()),
    }
}

/// Round a floating-point coordinate to the integer pixel value adb's `input`
/// commands require, rejecting non-finite values instead of silently truncating.
pub fn to_android_coordinate(value: f64) -> Result<i32, String> {
    if !value.is_finite() {
        return Err(format!("coordinate {value} is not a finite number"));
    }
    Ok(value.round() as i32)
}

/// tvOS/iOS/watchOS remote and hardware buttons reachable through ControlKit's
/// `device.io.button` method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppleButton {
    Up,
    Down,
    Left,
    Right,
    Select,
    Menu,
    Home,
    PlayPause,
}

impl AppleButton {
    pub const ALL: [&'static str; 8] = [
        "up",
        "down",
        "left",
        "right",
        "select",
        "menu",
        "home",
        "playPause",
    ];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "select" => Some(Self::Select),
            "menu" => Some(Self::Menu),
            "home" => Some(Self::Home),
            "playPause" => Some(Self::PlayPause),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Select => "select",
            Self::Menu => "menu",
            Self::Home => "home",
            Self::PlayPause => "playPause",
        }
    }
}

/// Android system buttons reachable through `adb shell input keyevent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidButton {
    Home,
    Back,
    Enter,
    Recents,
}

impl AndroidButton {
    pub const ALL: [&'static str; 4] = ["home", "back", "enter", "recents"];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "home" => Some(Self::Home),
            "back" => Some(Self::Back),
            "enter" => Some(Self::Enter),
            "recents" => Some(Self::Recents),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Back => "back",
            Self::Enter => "enter",
            Self::Recents => "recents",
        }
    }
}

/// The button-name vocabulary accepted by `input_button`, spanning both
/// platforms so the tool has a single typed, enumerable field instead of a free
/// string. Which subset is actually valid depends on the resolved target's
/// platform: Apple targets accept `up`, `down`, `left`, `right`, `select`,
/// `menu`, `home`, or `playPause`; Android targets accept `home`, `back`,
/// `enter`, or `recents`. `home` is valid on both. Passing a value from the
/// wrong platform's vocabulary for the resolved target is rejected at call time
/// with an error naming the valid set for that platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ButtonName {
    Up,
    Down,
    Left,
    Right,
    Select,
    Menu,
    Home,
    PlayPause,
    Back,
    Enter,
    Recents,
}

impl ButtonName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Select => "select",
            Self::Menu => "menu",
            Self::Home => "home",
            Self::PlayPause => "playPause",
            Self::Back => "back",
            Self::Enter => "enter",
            Self::Recents => "recents",
        }
    }
}

/// Screen orientation accepted by `orientation_set`. Apple-only: ControlKit is the
/// only backend that implements orientation control today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum Orientation {
    Portrait,
    Landscape,
}

impl Orientation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Portrait => "PORTRAIT",
            Self::Landscape => "LANDSCAPE",
        }
    }
}

/// Static, hand-maintained capability description for Android targets. Unlike
/// Apple's `device.capabilities` ControlKit call, adb has no capability-discovery
/// RPC, so this mirrors exactly the actions `AndroidDebugBridge` implements.
pub fn android_capabilities() -> serde_json::Value {
    serde_json::json!({
        "screen_capture": true,
        "app_launch": {
            "supported": true,
            "note": "Resolves the Leanback (Android TV) launcher activity first, \
                     then falls back to the standard launcher activity.",
        },
        "app_terminate": true,
        "url_open": true,
        "input_tap": true,
        "input_text": true,
        "input_swipe": true,
        "input_button": {
            "supported": true,
            "buttons": AndroidButton::ALL,
        },
        "app_install_launch": false,
        "ui_describe": false,
        "ui_element_list": false,
        "orientation_get": false,
        "orientation_set": false,
        "input_click": false,
        "input_spatial_tap": false,
    })
}

/// Target-only arguments shared by `device_select` and `device_capabilities`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeviceSelectArgs {
    /// Exact Apple simulator name to remember as the active target. Mutually
    /// exclusive with `host` and `android_serial`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID to remember as the active target. Mutually exclusive
    /// with `host` and `android_serial`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device, remote runner, or local Mac to
    /// remember. Mutually exclusive with `simulator_name`/`simulator_udid` and
    /// `android_serial`.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004. Only meaningful with
    /// `simulator_name`/`simulator_udid` or `host`.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Android device serial from `adb devices -l` to remember as the active
    /// target. Mutually exclusive with every Apple field.
    #[serde(default)]
    pub android_serial: Option<String>,
}

/// Structured result of `device_select`: the single unambiguous target that is
/// now remembered for later calls, until overridden by another `device_select`
/// call or by explicit target fields on a later call.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DeviceSelectOutput {
    pub selected: SelectedTarget,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeviceCapabilitiesArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device, remote runner, or local Mac.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Android device serial from `adb devices -l`.
    #[serde(default)]
    pub android_serial: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppInstallLaunchArgs {
    /// Exact simulator name to use. Provide this or `simulator_udid`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID to use. Provide this or `simulator_name`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// Path to the `.app` bundle on disk.
    pub app_path: PathBuf,
    /// Bundle identifier of the app inside `app_path`.
    pub bundle_id: String,
    /// Terminate a previously running instance of the app before launching it.
    #[serde(default)]
    pub terminate_before_launch: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScreenCaptureArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host. Screen capture is not implemented for ControlKit hosts;
    /// use `device` instead for a physical device, or select a simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Physical device identifier known to CoreDevice (UDID, ECID, serial number,
    /// user-provided name, or DNS name). Provide this to capture a paired physical
    /// Apple device instead of a simulator. Mutually exclusive with every other
    /// field.
    #[serde(default)]
    pub device: Option<String>,
    /// Android device serial from `adb devices -l`.
    #[serde(default)]
    pub android_serial: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppTargetArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Android device serial from `adb devices -l`.
    #[serde(default)]
    pub android_serial: Option<String>,
    /// App identifier: an Apple bundle identifier, or an Android package name,
    /// depending on the resolved target platform.
    pub app_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UrlOpenArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host. Opening URLs is not implemented for ControlKit hosts;
    /// use a simulator or an Android target instead.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Android device serial from `adb devices -l`.
    #[serde(default)]
    pub android_serial: Option<String>,
    /// HTTP(S) URL or custom URL scheme to open.
    pub url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TapArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Android device serial from `adb devices -l`.
    #[serde(default)]
    pub android_serial: Option<String>,
    /// Horizontal screen coordinate in points (Apple) or pixels (Android).
    pub x: f64,
    /// Vertical screen coordinate in points (Apple) or pixels (Android).
    pub y: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TextArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Android device serial from `adb devices -l`.
    #[serde(default)]
    pub android_serial: Option<String>,
    /// Single-line text to type into the focused field.
    pub text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SwipeArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Android device serial from `adb devices -l`.
    #[serde(default)]
    pub android_serial: Option<String>,
    /// Swipe start horizontal coordinate.
    pub x1: f64,
    /// Swipe start vertical coordinate.
    pub y1: f64,
    /// Swipe end horizontal coordinate.
    pub x2: f64,
    /// Swipe end vertical coordinate.
    pub y2: f64,
    /// Swipe duration in milliseconds. Defaults to 300. Android only; Apple
    /// ControlKit swipes ignore this field.
    #[serde(default)]
    pub duration_ms: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ButtonArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Android device serial from `adb devices -l`.
    #[serde(default)]
    pub android_serial: Option<String>,
    /// Button to press. For Apple targets: up, down, left, right, select, menu,
    /// home, or playPause. For Android targets: home, back, enter, or recents.
    /// Passing a value from the wrong platform's vocabulary is rejected with a
    /// clear error naming the valid set for the resolved target.
    pub button: ButtonName,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppleTargetArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner. Omit for a local
    /// simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UiTargetArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner. Omit for a local
    /// simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Bundle identifier of the app whose accessibility hierarchy should be read.
    #[serde(default)]
    pub bundle_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OrientationSetArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner. Omit for a local
    /// simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Desired screen orientation.
    pub orientation: Orientation,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinateArgs {
    /// Exact Apple simulator name. Omit to use the target selected with
    /// `device_select`.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Apple simulator UDID. Omit to use the target selected with `device_select`.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner. Omit for a local
    /// simulator or Mac.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Horizontal screen coordinate.
    pub x: f32,
    /// Vertical screen coordinate.
    pub y: f32,
}

#[derive(Debug, Default)]
pub struct XcrsMcpServer;

impl XcrsMcpServer {
    pub fn new() -> Self {
        Self
    }
}

#[macro_export]
macro_rules! xcrs_mcp_tools {
    (
        $server:ty,
        $device_list_name:literal,
        $device_select_name:literal,
        $device_capabilities_name:literal,
        $app_install_launch_name:literal,
        $screen_capture_name:literal,
        $app_launch_name:literal,
        $app_terminate_name:literal,
        $url_open_name:literal,
        $ui_describe_name:literal,
        $ui_element_list_name:literal,
        $input_tap_name:literal,
        $input_text_name:literal,
        $input_swipe_name:literal,
        $input_button_name:literal,
        $input_click_name:literal,
        $input_spatial_tap_name:literal,
        $orientation_get_name:literal,
        $orientation_set_name:literal
    ) => {
        #[::rmcp::tool_router(router = xcrs_tool_router, vis = "pub(crate)")]
        impl $server {
            fn selected_target_slot(
            ) -> &'static ::std::sync::Mutex<Option<$crate::mcp::SelectedTarget>> {
                static SLOT: ::std::sync::Mutex<Option<$crate::mcp::SelectedTarget>> =
                    ::std::sync::Mutex::new(None);
                &SLOT
            }

            fn stored_target() -> Option<$crate::mcp::SelectedTarget> {
                Self::selected_target_slot()
                    .lock()
                    .ok()
                    .and_then(|slot| slot.clone())
            }

            fn require_bundle_id(
                tool_name: &str,
                bundle_id: Option<String>,
            ) -> ::std::result::Result<String, ::rmcp::model::ErrorData> {
                let bundle_id = bundle_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|bundle_id| !bundle_id.is_empty())
                    .ok_or_else(|| {
                        ::rmcp::model::ErrorData::invalid_request(
                            format!(
                                "{tool_name} requires bundle_id. Pass the bundle identifier of the foreground Apple app."
                            ),
                            None,
                        )
                    })?;
                Ok(bundle_id.to_string())
            }

            /// Validate and tag a target from per-call fields, falling back to the
            /// target selected with `device_select` when no field is provided.
            fn dispatch_target(
                simulator_name: Option<String>,
                simulator_udid: Option<String>,
                host: Option<String>,
                controlkit_port: Option<u16>,
                android_serial: Option<String>,
            ) -> ::std::result::Result<$crate::mcp::SelectedTarget, ::rmcp::model::ErrorData>
            {
                $crate::mcp::resolve_selected_target(
                    simulator_name,
                    simulator_udid,
                    host,
                    controlkit_port,
                    android_serial,
                    Self::stored_target(),
                )
                .map_err(|error| ::rmcp::model::ErrorData::invalid_request(error, None))
            }

            /// Like [`Self::dispatch_target`] but for tools that only support Apple
            /// targets; rejects an Android target (explicit or stored) with a clear
            /// error naming `tool_name`.
            fn dispatch_apple_target(
                tool_name: &str,
                simulator_name: Option<String>,
                simulator_udid: Option<String>,
                host: Option<String>,
                controlkit_port: Option<u16>,
            ) -> ::std::result::Result<$crate::mcp::SelectedTarget, ::rmcp::model::ErrorData>
            {
                let target =
                    Self::dispatch_target(simulator_name, simulator_udid, host, controlkit_port, None)?;
                $crate::mcp::require_apple_target(tool_name, &target)
                    .map_err(|error| ::rmcp::model::ErrorData::invalid_request(error, None))?;
                Ok(target)
            }

            fn simulator_for(
                simulator_name: &Option<String>,
                simulator_udid: &Option<String>,
            ) -> ::std::result::Result<$crate::Simulator, ::rmcp::model::ErrorData> {
                let tools = $crate::XcodeCommandLineTools::new();
                match (simulator_udid, simulator_name) {
                    (Some(udid), _) => tools.simctl().find_simulator_by_udid(udid).map_err(
                        |error| ::rmcp::model::ErrorData::invalid_request(error.to_string(), None),
                    ),
                    (None, Some(name)) => tools.simctl().find_simulator_by_name(name).map_err(
                        |error| ::rmcp::model::ErrorData::invalid_request(error.to_string(), None),
                    ),
                    (None, None) => Err(::rmcp::model::ErrorData::invalid_request(
                        "Apple target requires simulator_name or simulator_udid.",
                        None,
                    )),
                }
            }

            /// Build the ControlKit client and, when applicable, resolve the
            /// simulator for an Apple target. Callers must not pass an Android
            /// target; use `dispatch_apple_target` or branch on the target first.
            fn controlkit_for_target(
                target: &$crate::mcp::SelectedTarget,
            ) -> ::std::result::Result<
                (Option<$crate::Simulator>, $crate::ControlKit),
                ::rmcp::model::ErrorData,
            > {
                match target {
                    $crate::mcp::SelectedTarget::AppleHost {
                        host,
                        controlkit_port,
                    } => Ok((
                        None,
                        $crate::ControlKit::with_host(host, controlkit_port.unwrap_or(12004)),
                    )),
                    $crate::mcp::SelectedTarget::AppleSimulator {
                        simulator_name,
                        simulator_udid,
                        controlkit_port,
                    } => {
                        let simulator = Self::simulator_for(simulator_name, simulator_udid)?;
                        Ok((
                            Some(simulator),
                            $crate::ControlKit::new(controlkit_port.unwrap_or(12004)),
                        ))
                    }
                    $crate::mcp::SelectedTarget::Android { .. } => {
                        Err(::rmcp::model::ErrorData::invalid_request(
                            "Android targets do not use ControlKit.",
                            None,
                        ))
                    }
                }
            }

            async fn run_android_blocking<T, F>(
                operation: F,
            ) -> ::anyhow::Result<T>
            where
                T: Send + 'static,
                F: FnOnce() -> ::anyhow::Result<T> + Send + 'static,
            {
                ::tokio::task::spawn_blocking(operation)
                    .await
                    .map_err(|error| ::anyhow::anyhow!("Android adb task failed: {error}"))?
            }

            async fn android_device_for(
                serial: Option<String>,
            ) -> ::std::result::Result<$crate::AndroidDevice, ::rmcp::model::ErrorData> {
                Self::run_android_blocking(move || {
                    $crate::AndroidDebugBridge::new().resolve_device(serial.as_deref())
                })
                .await
                .map_err(|error| {
                    ::rmcp::model::ErrorData::invalid_request(error.to_string(), None)
                })
            }

            fn target_label(
                target: &$crate::mcp::SelectedTarget,
                simulator: &Option<$crate::Simulator>,
            ) -> String {
                match target {
                    $crate::mcp::SelectedTarget::Android { serial } => serial
                        .clone()
                        .unwrap_or_else(|| "the connected Android device".to_string()),
                    $crate::mcp::SelectedTarget::AppleHost { host, .. } => host.clone(),
                    $crate::mcp::SelectedTarget::AppleSimulator { .. } => simulator
                        .as_ref()
                        .map(|simulator| simulator.name.clone())
                        .unwrap_or_else(|| "the selected simulator".to_string()),
                }
            }

            #[::rmcp::tool(
                name = $device_list_name,
                title = "List devices",
                annotations(title = "List devices", read_only_hint = true, idempotent_hint = true),
                description = "Purpose: enumerate every automation-capable device known to this machine in one normalized list, across Apple and Android. When to use vs siblings: call this first to discover what is available, then remember a choice with device_select; use device_capabilities afterwards to check what a specific target supports before driving it. Behavior: lists every Apple simulator known to Xcode via `simctl` (platform, runtime, UDID, availability, and boot state) plus every Android device or emulator known to `adb devices -l` (serial, model, and authorization/connection state), normalized into entries with platform, kind, identifier, name, state, and transport fields. Prerequisites: Xcode command line tools and/or adb must be installed and on PATH; neither is required for the other platform's entries to appear. Failure modes: if one platform's tool is missing or errors, that platform's entries are omitted and the error is reported under an `errors` object keyed by platform name, while the other platform's entries are still returned. Limitations: Apple physical (non-simulator) devices are not listed here because this crate has no reliable device-discovery command for them; only simulators are enumerated for Apple. Screenshot support for a paired physical Apple device still works through screen_capture's `device` parameter even though it will not appear in this list."
            )]
            async fn device_list(
                &self,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let mut devices = Vec::new();
                let mut errors = ::serde_json::Map::new();

                match $crate::XcodeCommandLineTools::new().simctl().list_simulators() {
                    Ok(simulators) => devices.extend(
                        simulators
                            .iter()
                            .map($crate::NormalizedDevice::from_simulator),
                    ),
                    Err(error) => {
                        errors.insert("apple".to_string(), ::serde_json::json!(error.to_string()));
                    }
                }

                match Self::run_android_blocking(|| {
                    $crate::AndroidDebugBridge::new().list_devices()
                })
                .await
                {
                    Ok(android_devices) => devices.extend(
                        android_devices
                            .iter()
                            .map($crate::NormalizedDevice::from_android_device),
                    ),
                    Err(error) => {
                        errors.insert("android".to_string(), ::serde_json::json!(error.to_string()));
                    }
                }

                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&::serde_json::json!({
                        "devices": devices,
                        "errors": errors,
                    }))?,
                ]))
            }

            #[::rmcp::tool(
                name = $device_select_name,
                title = "Select active device",
                annotations(title = "Select active device", read_only_hint = false, destructive_hint = false, idempotent_hint = true),
                output_schema = ::rmcp::handler::server::tool::schema_for_type::<$crate::mcp::DeviceSelectOutput>(),
                description = "Purpose: remember one target for later calls so they don't need repeated target arguments. When to use vs siblings: call this once after picking a target from device_list; every other action tool accepts the same target fields per-call and will use this stored choice only when none of them are passed, and any explicit field on a later call overrides the stored choice for that single call. Behavior: accepts exactly one of (a) simulator_name and/or simulator_udid for an Apple simulator, (b) host (with optional controlkit_port) for an Apple ControlKit endpoint such as a paired physical device, a remote runner, or a local Mac, or (c) android_serial for an Android device or emulator. Prerequisites: none beyond having a reachable target; this tool stores the selection without connecting to it. Failure modes: returns an error if no field is provided, or if fields from more than one of the three target kinds are combined (e.g. simulator_name with host, or android_serial with host)."
            )]
            async fn device_select(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::DeviceSelectArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = $crate::mcp::resolve_selected_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                    None,
                )
                .map_err(|error| ::rmcp::model::ErrorData::invalid_request(error, None))?;
                match Self::selected_target_slot().lock() {
                    Ok(mut slot) => *slot = Some(target.clone()),
                    Err(_) => {
                        return Err(::rmcp::model::ErrorData::internal_error(
                            "Failed to store the selected target.",
                            None,
                        ))
                    }
                }
                let output = $crate::mcp::DeviceSelectOutput { selected: target };
                let value = ::serde_json::to_value(&output).map_err(|error| {
                    ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                })?;
                Ok(::rmcp::model::CallToolResult::structured(value))
            }

            #[::rmcp::tool(
                name = $device_capabilities_name,
                title = "Get device capabilities",
                annotations(title = "Get device capabilities", read_only_hint = true, idempotent_hint = true),
                description = "Purpose: report which automation actions the resolved target supports before driving it. When to use vs siblings: call this after device_select (or with explicit target fields) and before UI actions, to confirm e.g. that orientation control or UI introspection is available on this target. Behavior: for an Apple target, forwards to the target's ControlKit `device.capabilities` JSON-RPC method and returns its raw result alongside the resolved simulator (if any); for an Android target, returns a static capability description reflecting exactly the adb actions this crate implements (screen capture, app launch/terminate, URL open, tap/text/swipe/button), since adb has no capability-discovery RPC. Prerequisites: for Apple, a reachable ControlKit endpoint (local companion app for a simulator, or the given host) must be running; for Android, adb must be able to reach the resolved device. Failure modes: returns an error if no target can be resolved, or if the ControlKit call fails or times out."
            )]
            async fn device_capabilities(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::DeviceCapabilitiesArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                match &target {
                    $crate::mcp::SelectedTarget::Android { .. } => {
                        Ok(::rmcp::model::CallToolResult::success(vec![
                            ::rmcp::model::ContentBlock::json(&::serde_json::json!({
                                "platform": "android",
                                "capabilities": $crate::mcp::android_capabilities(),
                            }))?,
                        ]))
                    }
                    _ => {
                        let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                        let result = controlkit
                            .call("device.capabilities", ::serde_json::json!({}))
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        Ok(::rmcp::model::CallToolResult::success(vec![
                            ::rmcp::model::ContentBlock::json(&::serde_json::json!({
                                "platform": "apple",
                                "simulator": simulator,
                                "capabilities": result,
                            }))?,
                        ]))
                    }
                }
            }

            #[::rmcp::tool(
                name = $app_install_launch_name,
                title = "Install and launch Apple simulator app",
                annotations(title = "Install and launch Apple simulator app", read_only_hint = false, destructive_hint = true, idempotent_hint = false),
                description = "Purpose: one-shot end-to-end setup for a freshly built Apple-platform app: boot an iOS, tvOS, watchOS, or visionOS simulator, install the .app bundle, optionally terminate a previous instance, launch it, and return its app container path. When to use vs siblings: use this once to get a build under test onto a simulator; use app_launch/app_terminate afterwards for an already-installed app, and screen_capture/ui_describe to inspect it. Behavior: boots the target simulator if it is not already booted, installs app_path, optionally force-terminates bundle_id first, launches bundle_id, then reads back the app's container directory. Prerequisites: Xcode command line tools installed; app_path must point to an existing .app bundle built for the selected simulator platform and architecture. Failure modes: errors if neither simulator_name nor simulator_udid is given, if the simulator cannot be found, if the app bundle targets a different platform, or if any underlying `simctl` step fails. Limitations: simulator-only; there is no Android or physical-device equivalent in this tool."
            )]
            async fn app_install_launch(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::AppInstallLaunchArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                if args.simulator_name.is_none() && args.simulator_udid.is_none() {
                    return Err(::rmcp::model::ErrorData::invalid_request(
                        "Provide either simulator_name or simulator_udid.",
                        None,
                    ));
                }

                let result = $crate::XcodeCommandLineTools::new()
                    .run_ios_app_test(&$crate::IosAppTest {
                        simulator_name: args.simulator_name,
                        simulator_udid: args.simulator_udid,
                        app_path: args.app_path,
                        bundle_id: args.bundle_id,
                        terminate_before_launch: args.terminate_before_launch,
                    })
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&result)?,
                ]))
            }

            #[::rmcp::tool(
                name = $screen_capture_name,
                title = "Capture screenshot",
                annotations(title = "Capture screenshot", read_only_hint = true, idempotent_hint = true),
                description = "Purpose: capture a PNG screenshot of the resolved target's current screen. When to use vs siblings: use this for a pixel image; use ui_describe or ui_element_list instead when you need element coordinates or labels rather than pixels. Behavior: for an Apple simulator, captures through `simctl io screenshot`; for a paired physical Apple device, pass its CoreDevice identifier as `device` to capture through `devicectl` (this bypasses simulator/host/android_serial resolution entirely and is mutually exclusive with them); for an Android device, captures through `adb exec-out screencap`. Prerequisites: the target must be booted/connected and, for simulators, `simctl` must be able to reach it. Failure modes: returns an error if `device` is combined with any other target field, if the resolved target is an Apple ControlKit host (screenshot over ControlKit is not implemented; use `device` or select a simulator instead), or if the underlying capture command fails."
            )]
            async fn screen_capture(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::ScreenCaptureArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                if let Some(device) = &args.device {
                    if args.simulator_name.is_some()
                        || args.simulator_udid.is_some()
                        || args.host.is_some()
                        || args.android_serial.is_some()
                    {
                        return Err(::rmcp::model::ErrorData::invalid_request(
                            "device cannot be combined with simulator_name, simulator_udid, host, or android_serial. Provide exactly one target.",
                            None,
                        ));
                    }
                    let screenshot = $crate::XcodeCommandLineTools::new()
                        .devicectl()
                        .screenshot(device)
                        .map_err(|error| {
                            ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                        })?;
                    return Ok(::rmcp::model::CallToolResult::success(vec![
                        ::rmcp::model::ContentBlock::image(
                            $crate::encode_base64(screenshot),
                            "image/png",
                        ),
                    ]));
                }

                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                let screenshot = match &target {
                    $crate::mcp::SelectedTarget::Android { serial } => {
                        let device = Self::android_device_for(serial.clone()).await?;
                        let device_serial = device.serial.clone();
                        Self::run_android_blocking(move || {
                            $crate::AndroidDebugBridge::new().screenshot(&device_serial)
                        })
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?
                    }
                    $crate::mcp::SelectedTarget::AppleSimulator {
                        simulator_name,
                        simulator_udid,
                        ..
                    } => {
                        let simulator = Self::simulator_for(simulator_name, simulator_udid)?;
                        $crate::XcodeCommandLineTools::new()
                            .simctl()
                            .screenshot(&simulator.udid)
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?
                    }
                    $crate::mcp::SelectedTarget::AppleHost { host, .. } => {
                        return Err(::rmcp::model::ErrorData::invalid_request(
                            format!(
                                "screen_capture over a ControlKit host ({host}) is not supported; pass the physical device's CoreDevice identifier as `device`, or select an Apple simulator or Android device instead."
                            ),
                            None,
                        ));
                    }
                };
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::image(
                        $crate::encode_base64(screenshot),
                        "image/png",
                    ),
                ]))
            }

            #[::rmcp::tool(
                name = $app_launch_name,
                title = "Launch app",
                annotations(title = "Launch app", read_only_hint = false, destructive_hint = false, idempotent_hint = true),
                description = "Purpose: launch an already-installed app by identifier on the resolved target. When to use vs siblings: use app_install_launch instead for a fresh iOS build that still needs installing; use this once the app is already on the device. Behavior: for an Apple simulator, launches through `simctl launch`; for an Apple ControlKit host, calls `device.apps.launch`; for Android, resolves (or prefers, if given) the launcher activity — trying the Android TV Leanback launcher category first, then the standard launcher category — and starts it. Prerequisites: app_id must already be installed on the resolved target. Failure modes: errors if no target can be resolved, if the app is not installed, or (Android) if no launchable activity is found for app_id."
            )]
            async fn app_launch(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::AppTargetArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                let label = match &target {
                    $crate::mcp::SelectedTarget::Android { serial } => {
                        let device = Self::android_device_for(serial.clone()).await?;
                        let device_serial = device.serial.clone();
                        let app_id = args.app_id.clone();
                        Self::run_android_blocking(move || {
                            $crate::AndroidDebugBridge::new()
                                .launch_app(&device_serial, &app_id)
                        })
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        device.serial
                    }
                    $crate::mcp::SelectedTarget::AppleHost {
                        host,
                        controlkit_port,
                    } => {
                        let controlkit =
                            $crate::ControlKit::with_host(host, controlkit_port.unwrap_or(12004));
                        controlkit
                            .call(
                                "device.apps.launch",
                                ::serde_json::json!({ "bundleId": args.app_id }),
                            )
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        host.clone()
                    }
                    $crate::mcp::SelectedTarget::AppleSimulator {
                        simulator_name,
                        simulator_udid,
                        ..
                    } => {
                        let simulator = Self::simulator_for(simulator_name, simulator_udid)?;
                        $crate::XcodeCommandLineTools::new()
                            .simctl()
                            .launch_app(&simulator.udid, &args.app_id)
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        simulator.name
                    }
                };
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Launched {} on {}.",
                        args.app_id, label
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $app_terminate_name,
                title = "Terminate app",
                annotations(title = "Terminate app", read_only_hint = false, destructive_hint = false, idempotent_hint = true),
                description = "Purpose: force-stop an installed app by identifier on the resolved target. When to use vs siblings: pair with app_launch to reset app state between test steps. Behavior: for an Apple simulator, terminates through `simctl terminate`; for an Apple ControlKit host, calls `device.apps.terminate`; for Android, force-stops through `am force-stop`. Prerequisites: none beyond a reachable target; terminating an app that is not running is not an error. Failure modes: errors if no target can be resolved or the underlying command fails."
            )]
            async fn app_terminate(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::AppTargetArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                let label = match &target {
                    $crate::mcp::SelectedTarget::Android { serial } => {
                        let device = Self::android_device_for(serial.clone()).await?;
                        let device_serial = device.serial.clone();
                        let app_id = args.app_id.clone();
                        Self::run_android_blocking(move || {
                            $crate::AndroidDebugBridge::new()
                                .terminate_app(&device_serial, &app_id)
                        })
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        device.serial
                    }
                    $crate::mcp::SelectedTarget::AppleHost {
                        host,
                        controlkit_port,
                    } => {
                        let controlkit =
                            $crate::ControlKit::with_host(host, controlkit_port.unwrap_or(12004));
                        controlkit
                            .call(
                                "device.apps.terminate",
                                ::serde_json::json!({ "bundleId": args.app_id }),
                            )
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        host.clone()
                    }
                    $crate::mcp::SelectedTarget::AppleSimulator {
                        simulator_name,
                        simulator_udid,
                        ..
                    } => {
                        let simulator = Self::simulator_for(simulator_name, simulator_udid)?;
                        $crate::XcodeCommandLineTools::new()
                            .simctl()
                            .terminate_app(&simulator.udid, &args.app_id)
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        simulator.name
                    }
                };
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Terminated {} on {}.",
                        args.app_id, label
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $url_open_name,
                title = "Open URL",
                annotations(title = "Open URL", read_only_hint = false, destructive_hint = false, idempotent_hint = false),
                description = "Purpose: open an HTTP(S) URL or custom URL scheme on the resolved target, e.g. to deep-link into an app. When to use vs siblings: use this instead of app_launch when you need to hand the app a specific URL/deep link rather than just foreground it. Behavior: for an Apple simulator, opens through `simctl openurl`; for Android, opens through `am start -a android.intent.action.VIEW`. Prerequisites: the target must have an app installed that can handle the URL/scheme. Failure modes: errors if no target can be resolved, or if the resolved target is an Apple ControlKit host (opening URLs over ControlKit is not implemented; use a simulator or an Android target instead)."
            )]
            async fn url_open(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::UrlOpenArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                let label = match &target {
                    $crate::mcp::SelectedTarget::Android { serial } => {
                        let device = Self::android_device_for(serial.clone()).await?;
                        let device_serial = device.serial.clone();
                        let url = args.url.clone();
                        Self::run_android_blocking(move || {
                            $crate::AndroidDebugBridge::new().open_url(&device_serial, &url)
                        })
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        device.serial
                    }
                    $crate::mcp::SelectedTarget::AppleSimulator {
                        simulator_name,
                        simulator_udid,
                        ..
                    } => {
                        let simulator = Self::simulator_for(simulator_name, simulator_udid)?;
                        $crate::XcodeCommandLineTools::new()
                            .simctl()
                            .open_url(&simulator.udid, &args.url)
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        simulator.name
                    }
                    $crate::mcp::SelectedTarget::AppleHost { host, .. } => {
                        return Err(::rmcp::model::ErrorData::invalid_request(
                            format!(
                                "url_open over a ControlKit host ({host}) is not supported; use a simulator or an Android target instead."
                            ),
                            None,
                        ));
                    }
                };
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Opened {} on {}.",
                        args.url, label
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $ui_describe_name,
                title = "Describe UI",
                annotations(title = "Describe UI", read_only_hint = true, idempotent_hint = true),
                description = "Purpose: return the full accessibility hierarchy of an Apple app as JSON. When to use vs siblings: use this to see everything on screen before tapping or typing; prefer ui_element_list when you only need actionable elements and their tap coordinates, since it is smaller and already filtered. Behavior: attaches to bundle_id through the resolved target's ControlKit `device.dump.ui` method and returns the raw hierarchy alongside the resolved simulator (if any). Prerequisites: a reachable ControlKit endpoint built from a version that implements `device.dump.ui`; bundle_id must identify an installed app. Failure modes: errors if no target can be resolved, if the resolved target is Android (Apple-only tool; ControlKit UI introspection has no Android equivalent), if bundle_id is invalid, or if the ControlKit runner is outdated or unavailable."
            )]
            async fn ui_describe(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::UiTargetArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let bundle_id = Self::require_bundle_id($ui_describe_name, args.bundle_id)?;
                let target = Self::dispatch_apple_target(
                    $ui_describe_name,
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                )?;
                let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                let result = controlkit
                    .call(
                        "device.dump.ui",
                        ::serde_json::json!({
                            "format": "json",
                            "bundleId": bundle_id,
                        }),
                    )
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&::serde_json::json!({
                        "simulator": simulator,
                        "ui": result,
                    }))?,
                ]))
            }

            #[::rmcp::tool(
                name = $ui_element_list_name,
                title = "List UI elements",
                annotations(title = "List UI elements", read_only_hint = true, idempotent_hint = true),
                description = "Purpose: list just the actionable accessibility elements of an Apple app with their labels and tap coordinates. When to use vs siblings: use this to decide where to tap; use ui_describe when you need the full hierarchy instead of a filtered, flatter list. Behavior: attaches to bundle_id through the resolved target's ControlKit `device.dump.ui` method, then filters to elements that have both a visible rect and an identifying label/name/value/rawIdentifier. Prerequisites: a reachable ControlKit endpoint built from a version that implements `device.dump.ui`; bundle_id must identify an installed app. Failure modes: errors if no target can be resolved, if the resolved target is Android (Apple-only tool), if bundle_id is invalid, or if the ControlKit runner is outdated or unavailable."
            )]
            async fn ui_element_list(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::UiTargetArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let bundle_id = Self::require_bundle_id($ui_element_list_name, args.bundle_id)?;
                let target = Self::dispatch_apple_target(
                    $ui_element_list_name,
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                )?;
                let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                let ui = controlkit
                    .call(
                        "device.dump.ui",
                        ::serde_json::json!({
                            "format": "json",
                            "bundleId": bundle_id,
                        }),
                    )
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                let elements = $crate::extract_controlkit_elements(&ui);
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&::serde_json::json!({
                        "simulator": simulator,
                        "elements": elements,
                    }))?,
                ]))
            }

            #[::rmcp::tool(
                name = $input_tap_name,
                title = "Tap (iOS/tvOS/Android)",
                annotations(title = "Tap (iOS/tvOS/Android)", read_only_hint = false, destructive_hint = false, idempotent_hint = false),
                description = "Purpose: tap the resolved target at screen coordinates. When to use vs siblings: use input_click for macOS and input_spatial_tap for visionOS instead; read coordinates from ui_element_list or ui_describe first. Behavior: for an Apple target, calls ControlKit `device.io.tap`; for Android, runs `adb shell input tap`, rounding x/y to the nearest pixel. Prerequisites: a reachable ControlKit endpoint (Apple) or adb connection (Android). Failure modes: errors if no target can be resolved, if x or y is not a finite number, or if the underlying call fails."
            )]
            async fn input_tap(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::TapArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                let label = match &target {
                    $crate::mcp::SelectedTarget::Android { serial } => {
                        let device = Self::android_device_for(serial.clone()).await?;
                        let x = $crate::mcp::to_android_coordinate(args.x).map_err(|error| {
                            ::rmcp::model::ErrorData::invalid_request(error, None)
                        })?;
                        let y = $crate::mcp::to_android_coordinate(args.y).map_err(|error| {
                            ::rmcp::model::ErrorData::invalid_request(error, None)
                        })?;
                        let device_serial = device.serial.clone();
                        Self::run_android_blocking(move || {
                            $crate::AndroidDebugBridge::new().tap(&device_serial, x, y)
                        })
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        device.serial
                    }
                    _ => {
                        let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                        controlkit
                            .call(
                                "device.io.tap",
                                ::serde_json::json!({ "x": args.x, "y": args.y }),
                            )
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        Self::target_label(&target, &simulator)
                    }
                };
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Tapped ({}, {}) on {}.",
                        args.x, args.y, label
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $input_text_name,
                title = "Type text",
                annotations(title = "Type text", read_only_hint = false, destructive_hint = false, idempotent_hint = false),
                description = "Purpose: type single-line text into the focused field of the resolved target. When to use vs siblings: tap the field first with input_tap to focus it, then use this to enter text. Behavior: for an Apple target, calls ControlKit `device.io.text`; for Android, runs `adb shell input text`, encoding spaces and rejecting the literal sequence `%s` (which adb's input command cannot express safely). Prerequisites: a focused text field on the target. Failure modes: errors if no target can be resolved, if text is empty or multi-line, or (Android) if text contains the literal sequence `%s`."
            )]
            async fn input_text(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::TextArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                let label = match &target {
                    $crate::mcp::SelectedTarget::Android { serial } => {
                        let device = Self::android_device_for(serial.clone()).await?;
                        let device_serial = device.serial.clone();
                        let text = args.text.clone();
                        Self::run_android_blocking(move || {
                            $crate::AndroidDebugBridge::new()
                                .type_text(&device_serial, &text)
                        })
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::invalid_request(error.to_string(), None)
                            })?;
                        device.serial
                    }
                    _ => {
                        let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                        controlkit
                            .call("device.io.text", ::serde_json::json!({ "text": args.text }))
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        Self::target_label(&target, &simulator)
                    }
                };
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!("Typed text on {}.", label)),
                ]))
            }

            #[::rmcp::tool(
                name = $input_swipe_name,
                title = "Swipe",
                annotations(title = "Swipe", read_only_hint = false, destructive_hint = false, idempotent_hint = false),
                description = "Purpose: swipe between two screen coordinates on the resolved target, e.g. to scroll. When to use vs siblings: use input_tap for a single point of contact instead. Behavior: for an Apple target, calls ControlKit `device.io.swipe`; for Android, runs `adb shell input swipe` with duration_ms (default 300, rounding x/y to the nearest pixel). Prerequisites: a reachable ControlKit endpoint (Apple) or adb connection (Android). Failure modes: errors if no target can be resolved, if any coordinate is not a finite number, or (Android) if duration_ms exceeds 10000."
            )]
            async fn input_swipe(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SwipeArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                let label = match &target {
                    $crate::mcp::SelectedTarget::Android { serial } => {
                        let device = Self::android_device_for(serial.clone()).await?;
                        let x1 = $crate::mcp::to_android_coordinate(args.x1).map_err(|error| {
                            ::rmcp::model::ErrorData::invalid_request(error, None)
                        })?;
                        let y1 = $crate::mcp::to_android_coordinate(args.y1).map_err(|error| {
                            ::rmcp::model::ErrorData::invalid_request(error, None)
                        })?;
                        let x2 = $crate::mcp::to_android_coordinate(args.x2).map_err(|error| {
                            ::rmcp::model::ErrorData::invalid_request(error, None)
                        })?;
                        let y2 = $crate::mcp::to_android_coordinate(args.y2).map_err(|error| {
                            ::rmcp::model::ErrorData::invalid_request(error, None)
                        })?;
                        let device_serial = device.serial.clone();
                        let duration_ms = args.duration_ms.unwrap_or(300);
                        Self::run_android_blocking(move || {
                            $crate::AndroidDebugBridge::new()
                                .swipe(&device_serial, x1, y1, x2, y2, duration_ms)
                        })
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::invalid_request(error.to_string(), None)
                            })?;
                        device.serial
                    }
                    _ => {
                        let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                        controlkit
                            .call(
                                "device.io.swipe",
                                ::serde_json::json!({
                                    "x1": args.x1,
                                    "y1": args.y1,
                                    "x2": args.x2,
                                    "y2": args.y2,
                                }),
                            )
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        Self::target_label(&target, &simulator)
                    }
                };
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!("Swiped on {}.", label)),
                ]))
            }

            #[::rmcp::tool(
                name = $input_button_name,
                title = "Press button",
                annotations(title = "Press button", read_only_hint = false, destructive_hint = false, idempotent_hint = false),
                description = "Purpose: press a hardware or remote button on the resolved target. When to use vs siblings: use this instead of input_tap for physical/remote controls such as Home or a tvOS remote's directional pad. Behavior: for an Apple target, validates button against up/down/left/right/select/menu/home/playPause and calls ControlKit `device.io.button`; for Android, validates button against home/back/enter/recents and runs the matching `adb shell input keyevent`. Prerequisites: a reachable ControlKit endpoint (Apple) or adb connection (Android). Failure modes: errors if no target can be resolved, or if button is not in the vocabulary for the resolved target's platform (the two platforms use different button names)."
            )]
            async fn input_button(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::ButtonArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_target(
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                    args.android_serial,
                )?;
                let label = match &target {
                    $crate::mcp::SelectedTarget::Android { serial } => {
                        let button = $crate::mcp::AndroidButton::parse(args.button.as_str()).ok_or_else(|| {
                            ::rmcp::model::ErrorData::invalid_request(
                                format!(
                                    "button must be one of {} for an Android target",
                                    $crate::mcp::AndroidButton::ALL.join(", ")
                                ),
                                None,
                            )
                        })?;
                        let device = Self::android_device_for(serial.clone()).await?;
                        let device_serial = device.serial.clone();
                        Self::run_android_blocking(move || {
                            $crate::AndroidDebugBridge::new()
                                .press_button(&device_serial, button.as_str())
                        })
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::invalid_request(error.to_string(), None)
                            })?;
                        device.serial
                    }
                    _ => {
                        let button = $crate::mcp::AppleButton::parse(args.button.as_str()).ok_or_else(|| {
                            ::rmcp::model::ErrorData::invalid_request(
                                format!(
                                    "button must be one of {} for an Apple target",
                                    $crate::mcp::AppleButton::ALL.join(", ")
                                ),
                                None,
                            )
                        })?;
                        let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                        controlkit
                            .call(
                                "device.io.button",
                                ::serde_json::json!({ "button": button.as_str() }),
                            )
                            .await
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                            })?;
                        Self::target_label(&target, &simulator)
                    }
                };
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Pressed {} on {}.",
                        args.button.as_str(), label
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $input_click_name,
                title = "Click (macOS)",
                annotations(title = "Click (macOS)", read_only_hint = false, destructive_hint = false, idempotent_hint = false),
                description = "Purpose: click the running macOS app at screen coordinates. When to use vs siblings: use input_tap for iOS/tvOS/watchOS and input_spatial_tap for visionOS instead. Behavior: calls the resolved target's ControlKit `device.io.click` method. Prerequisites: a reachable ControlKit endpoint on a macOS target. Failure modes: errors if no target can be resolved, or if the resolved target is Android (Apple-only tool)."
            )]
            async fn input_click(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::CoordinateArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_apple_target(
                    $input_click_name,
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                )?;
                let (_, controlkit) = Self::controlkit_for_target(&target)?;
                controlkit
                    .call(
                        "device.io.click",
                        ::serde_json::json!({ "x": args.x, "y": args.y }),
                    )
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Clicked ({}, {}).",
                        args.x, args.y
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $input_spatial_tap_name,
                title = "Spatial tap (visionOS)",
                annotations(title = "Spatial tap (visionOS)", read_only_hint = false, destructive_hint = false, idempotent_hint = false),
                description = "Purpose: perform a spatial tap on the running visionOS app at screen coordinates. When to use vs siblings: use input_tap for iOS/tvOS/watchOS and input_click for macOS instead. Behavior: calls the resolved target's ControlKit `device.io.spatial.tap` method. Prerequisites: a reachable ControlKit endpoint on a visionOS target. Failure modes: errors if no target can be resolved, or if the resolved target is Android (Apple-only tool)."
            )]
            async fn input_spatial_tap(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::CoordinateArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_apple_target(
                    $input_spatial_tap_name,
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                )?;
                let (_, controlkit) = Self::controlkit_for_target(&target)?;
                controlkit
                    .call(
                        "device.io.spatial.tap",
                        ::serde_json::json!({ "x": args.x, "y": args.y }),
                    )
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Performed spatial tap at ({}, {}).",
                        args.x, args.y
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $orientation_get_name,
                title = "Get orientation",
                annotations(title = "Get orientation", read_only_hint = true, idempotent_hint = true),
                description = "Purpose: report the resolved target's current screen orientation. When to use vs siblings: pair with orientation_set to check the result of a rotation. Behavior: calls the resolved target's ControlKit `device.io.orientation.get` method and returns its raw result alongside the resolved simulator (if any). Prerequisites: a reachable ControlKit endpoint. Failure modes: errors if no target can be resolved, or if the resolved target is Android (Apple-only tool; adb has no orientation query)."
            )]
            async fn orientation_get(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::AppleTargetArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_apple_target(
                    $orientation_get_name,
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                )?;
                let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                let result = controlkit
                    .call("device.io.orientation.get", ::serde_json::json!({}))
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&::serde_json::json!({
                        "simulator": simulator,
                        "orientation": result,
                    }))?,
                ]))
            }

            #[::rmcp::tool(
                name = $orientation_set_name,
                title = "Set orientation",
                annotations(title = "Set orientation", read_only_hint = false, destructive_hint = false, idempotent_hint = true),
                description = "Purpose: set the resolved target's screen orientation. When to use vs siblings: pair with orientation_get to confirm the result. Behavior: calls the resolved target's ControlKit `device.io.orientation.set` method with PORTRAIT or LANDSCAPE. Prerequisites: a reachable ControlKit endpoint. Failure modes: errors if no target can be resolved, or if the resolved target is Android (Apple-only tool; adb has no orientation control)."
            )]
            async fn orientation_set(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::OrientationSetArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = Self::dispatch_apple_target(
                    $orientation_set_name,
                    args.simulator_name,
                    args.simulator_udid,
                    args.host,
                    args.controlkit_port,
                )?;
                let (simulator, controlkit) = Self::controlkit_for_target(&target)?;
                controlkit
                    .call(
                        "device.io.orientation.set",
                        ::serde_json::json!({ "orientation": args.orientation.as_str() }),
                    )
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Set orientation to {} on {}.",
                        args.orientation.as_str(),
                        Self::target_label(&target, &simulator)
                    )),
                ]))
            }
        }
    };
}

xcrs_mcp_tools!(
    XcrsMcpServer,
    "device_list",
    "device_select",
    "device_capabilities",
    "app_install_launch",
    "screen_capture",
    "app_launch",
    "app_terminate",
    "url_open",
    "ui_describe",
    "ui_element_list",
    "input_tap",
    "input_text",
    "input_swipe",
    "input_button",
    "input_click",
    "input_spatial_tap",
    "orientation_get",
    "orientation_set"
);

#[rmcp::tool_handler(router = Self::xcrs_tool_router())]
impl ServerHandler for XcrsMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut implementation = Implementation::from_build_env();
        implementation.name = "xcrs".to_string();
        implementation.version = env!("CARGO_PKG_VERSION").to_string();

        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(implementation)
            .with_instructions(
                "xcrs exposes a canonical cross-platform automation contract (device_list, \
                 device_select, device_capabilities, app_install_launch, screen_capture, \
                 app_launch, app_terminate, url_open, ui_describe, ui_element_list, input_tap, \
                 input_text, input_swipe, input_button, input_click, input_spatial_tap, \
                 orientation_get, orientation_set) over Xcode/simctl/devicectl/ControlKit for \
                 Apple platforms and adb for Android.",
            )
    }
}

pub async fn serve() -> Result<()> {
    let running = XcrsMcpServer::new()
        .serve(stdio())
        .await
        .map_err(|error| anyhow!("Failed to start xcrs MCP server: {error}"))?;
    running
        .waiting()
        .await
        .map_err(|error| anyhow!("xcrs MCP server stopped unexpectedly: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 18 canonical tool names shared by the standalone and embedded
    /// automation servers.
    const CANONICAL_TOOL_NAMES: [&str; 18] = [
        "device_list",
        "device_select",
        "device_capabilities",
        "app_install_launch",
        "screen_capture",
        "app_launch",
        "app_terminate",
        "url_open",
        "ui_describe",
        "ui_element_list",
        "input_tap",
        "input_text",
        "input_swipe",
        "input_button",
        "input_click",
        "input_spatial_tap",
        "orientation_get",
        "orientation_set",
    ];

    /// Fully-qualified tool names this crate used before the 18-tool
    /// consolidation (e.g. `xcrs_list_simulators`, `xcrs_use_target`,
    /// `xcrs_gesture`). None of these must reappear as a canonical name; a
    /// match here means a rename regressed back to pre-consolidation naming.
    const SUPERSEDED_TOOL_NAMES: [&str; 19] = [
        "xcrs_list_simulators",
        "xcrs_find_simulator",
        "xcrs_boot_and_install",
        "xcrs_screenshot",
        "xcrs_launch_app",
        "xcrs_terminate_app",
        "xcrs_open_url",
        "xcrs_describe_ui",
        "xcrs_list_elements",
        "xcrs_tap",
        "xcrs_type_text",
        "xcrs_swipe",
        "xcrs_button",
        "xcrs_capabilities",
        "xcrs_click",
        "xcrs_gesture",
        "xcrs_use_target",
        "xcrs_orientation_get",
        "xcrs_orientation_set",
    ];

    /// Tool names whose output is guaranteed to be either image content
    /// (`screen_capture`), plain text with no structured payload, or a
    /// passthrough of an arbitrary external JSON-RPC result (ControlKit's
    /// `device.capabilities`/`device.dump.ui`/`device.io.orientation.get`, or
    /// simulator/device metadata types owned outside this module). None of
    /// these can be given a truthful, non-generic `output_schema` without
    /// either lying about their shape or coupling this module's schema to
    /// types it does not own, so they intentionally advertise no
    /// `output_schema`. Only `device_select` returns structured content built
    /// entirely from types this module owns, so it is the only tool with one.
    const TOOLS_WITHOUT_OUTPUT_SCHEMA: [&str; 17] = [
        "device_list",
        "device_capabilities",
        "app_install_launch",
        "screen_capture",
        "app_launch",
        "app_terminate",
        "url_open",
        "ui_describe",
        "ui_element_list",
        "input_tap",
        "input_text",
        "input_swipe",
        "input_button",
        "input_click",
        "input_spatial_tap",
        "orientation_get",
        "orientation_set",
    ];

    #[test]
    fn tool_router_exposes_exactly_the_eighteen_canonical_tools() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();

        assert_eq!(
            tools.len(),
            CANONICAL_TOOL_NAMES.len(),
            "expected exactly {} canonical tools, found {}",
            CANONICAL_TOOL_NAMES.len(),
            tools.len()
        );

        let mut actual_names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
        actual_names.sort_unstable();
        let mut expected_names = CANONICAL_TOOL_NAMES;
        expected_names.sort_unstable();
        assert_eq!(actual_names, expected_names);
    }

    #[test]
    fn tool_router_uses_unprefixed_canonical_names_with_no_legacy_names() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        for tool in &tools {
            assert!(
                !tool.name.starts_with("xcrs_") && !tool.name.starts_with("smb_"),
                "{} should not repeat a server prefix",
                tool.name
            );
            assert!(
                !SUPERSEDED_TOOL_NAMES.contains(&tool.name.as_ref()),
                "{} is a superseded pre-consolidation tool name",
                tool.name
            );
        }
    }

    #[test]
    fn every_tool_has_a_meaningful_concise_title() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        for tool in &tools {
            let title = tool
                .title
                .as_deref()
                .unwrap_or_else(|| panic!("{} is missing a top-level title", tool.name));
            assert!(!title.trim().is_empty(), "{} has a blank title", tool.name);
            assert!(
                title.len() <= 40,
                "{} title should be concise (<= 40 chars), got {title:?}",
                tool.name
            );
            assert_ne!(
                title,
                tool.name.as_ref(),
                "{} title should be a human-readable label, not the raw tool name",
                tool.name
            );
        }
    }

    #[test]
    fn every_tool_has_a_front_loaded_transparent_description() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        for tool in &tools {
            let description = tool
                .description
                .as_deref()
                .unwrap_or_else(|| panic!("{} is missing a description", tool.name));
            assert!(
                !description.trim().is_empty(),
                "{} has a blank description",
                tool.name
            );
            // Purpose Clarity: the purpose must be the first thing a reader sees.
            assert!(
                description.starts_with("Purpose:"),
                "{} description should front-load its purpose",
                tool.name
            );
            // Usage Guidelines: siblings and when-to-use guidance must be present.
            assert!(
                description.contains("When to use"),
                "{} description should say when to use it relative to sibling tools",
                tool.name
            );
            // Behavioral Transparency: behavior, prerequisites, and failure modes.
            assert!(
                description.contains("Behavior:"),
                "{} description should document its behavior",
                tool.name
            );
            assert!(
                description.contains("Prerequisites:"),
                "{} description should document its prerequisites",
                tool.name
            );
            assert!(
                description.contains("Failure modes:"),
                "{} description should document its failure modes",
                tool.name
            );
            // Not a snapshot: only bound length loosely, so wording can evolve
            // without brittle exact-text assertions.
            assert!(
                description.len() >= 150,
                "{} description is too terse for behavioral transparency ({} chars)",
                tool.name,
                description.len()
            );
        }
    }

    #[test]
    fn every_tool_has_internally_consistent_annotations() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        for tool in &tools {
            let annotations = tool
                .annotations
                .as_ref()
                .unwrap_or_else(|| panic!("{} is missing annotations", tool.name));
            let annotations_title = annotations
                .title
                .as_deref()
                .unwrap_or_else(|| panic!("{} annotations are missing a title", tool.name));
            assert_eq!(
                Some(annotations_title),
                tool.title.as_deref(),
                "{} top-level title and annotations title must agree",
                tool.name
            );
            // A tool cannot simultaneously claim to be read-only and to
            // perform destructive updates.
            assert!(
                !(annotations.read_only_hint == Some(true)
                    && annotations.destructive_hint == Some(true)),
                "{} annotations contradict: read_only_hint and destructive_hint are both true",
                tool.name
            );
        }
    }

    #[test]
    fn every_input_schema_property_has_a_description() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        for tool in &tools {
            let Some(properties) = tool
                .input_schema
                .get("properties")
                .and_then(|value| value.as_object())
            else {
                continue;
            };
            for (property_name, property_schema) in properties {
                let description = property_schema
                    .get("description")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                assert!(
                    !description.trim().is_empty(),
                    "{}.{} is missing a parameter description",
                    tool.name,
                    property_name
                );
            }
        }
    }

    #[test]
    fn ui_tools_keep_bundle_id_optional_in_the_input_schema() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        for tool_name in ["ui_describe", "ui_element_list"] {
            let tool = tools
                .iter()
                .find(|tool| tool.name.as_ref() == tool_name)
                .unwrap_or_else(|| panic!("{tool_name} should be registered"));
            let required = tool
                .input_schema
                .get("required")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();

            assert!(tool.input_schema["properties"].get("bundle_id").is_some());
            assert!(!required.contains(&serde_json::json!("bundle_id")));
        }
    }

    #[test]
    fn orientation_get_does_not_advertise_bundle_id() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        let tool = tools
            .iter()
            .find(|tool| tool.name.as_ref() == "orientation_get")
            .expect("orientation_get should be registered");
        let properties = tool.input_schema["properties"]
            .as_object()
            .expect("orientation_get properties should be an object");

        for property in [
            "simulator_name",
            "simulator_udid",
            "host",
            "controlkit_port",
        ] {
            assert!(
                properties.contains_key(property),
                "orientation_get should advertise {property}"
            );
        }
        assert!(!properties.contains_key("bundle_id"));
    }

    #[test]
    fn ui_target_args_accept_omitted_bundle_id_and_validate_it_explicitly() {
        let args: UiTargetArgs =
            serde_json::from_value(serde_json::json!({})).expect("arguments should deserialize");

        assert!(args.bundle_id.is_none());
        let error = XcrsMcpServer::require_bundle_id("ui_describe", args.bundle_id)
            .expect_err("missing bundle_id should fail validation");
        assert!(error.to_string().contains("ui_describe requires bundle_id"));
        assert_eq!(
            XcrsMcpServer::require_bundle_id(
                "ui_describe",
                Some("  com.example.app  ".to_string())
            )
            .expect("non-empty bundle_id should pass validation"),
            "com.example.app"
        );
    }

    #[test]
    fn output_schema_is_present_only_where_structured_content_is_guaranteed() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        assert_eq!(
            tools.len(),
            TOOLS_WITHOUT_OUTPUT_SCHEMA.len() + 1,
            "update TOOLS_WITHOUT_OUTPUT_SCHEMA if the canonical tool set changes"
        );
        for tool in &tools {
            if tool.name.as_ref() == "device_select" {
                assert!(
                    tool.output_schema.is_some(),
                    "device_select should advertise an output_schema for its fully-owned SelectedTarget payload"
                );
            } else {
                assert!(
                    TOOLS_WITHOUT_OUTPUT_SCHEMA.contains(&tool.name.as_ref()),
                    "{} has an output_schema but is not in the documented intentional-absence list",
                    tool.name
                );
                assert!(
                    tool.output_schema.is_none(),
                    "{} should not advertise an output_schema (image, text-only, or externally-owned/passthrough content)",
                    tool.name
                );
            }
        }
    }

    #[tokio::test]
    async fn device_select_structured_content_matches_its_advertised_output_schema() {
        let tools = XcrsMcpServer::xcrs_tool_router().list_all();
        let device_select_tool = tools
            .iter()
            .find(|tool| tool.name.as_ref() == "device_select")
            .expect("device_select tool should be registered");
        let output_schema = device_select_tool
            .output_schema
            .as_ref()
            .expect("device_select should have an output_schema");
        let required_properties: Vec<&str> = output_schema
            .get("required")
            .and_then(|value| value.as_array())
            .map(|values| values.iter().filter_map(|value| value.as_str()).collect())
            .unwrap_or_default();

        let result = XcrsMcpServer::new()
            .device_select(::rmcp::handler::server::wrapper::Parameters(
                DeviceSelectArgs {
                    simulator_name: Some("iPhone 16".to_string()),
                    simulator_udid: None,
                    host: None,
                    controlkit_port: None,
                    android_serial: None,
                },
            ))
            .await
            .expect("device_select should succeed with an unambiguous simulator target");

        let structured_content = result
            .structured_content
            .expect("device_select should populate structured_content matching its output_schema");
        let structured_object = structured_content
            .as_object()
            .expect("structured_content should be a JSON object");
        for property in &required_properties {
            assert!(
                structured_object.contains_key(*property),
                "structured_content is missing required property {property}"
            );
        }
        assert_eq!(
            structured_content["selected"]["platform"],
            serde_json::json!("apple_simulator")
        );
    }

    fn apple_simulator(name: &str) -> SelectedTarget {
        SelectedTarget::AppleSimulator {
            simulator_name: Some(name.to_string()),
            simulator_udid: None,
            controlkit_port: None,
        }
    }

    #[test]
    fn resolves_android_target_from_explicit_serial() {
        let target = resolve_selected_target(
            None,
            None,
            None,
            None,
            Some("emulator-5554".to_string()),
            None,
        )
        .expect("target should resolve");
        assert_eq!(
            target,
            SelectedTarget::Android {
                serial: Some("emulator-5554".to_string())
            }
        );
    }

    #[test]
    fn resolves_apple_simulator_target_from_explicit_name() {
        let target =
            resolve_selected_target(Some("iPhone 16".to_string()), None, None, None, None, None)
                .expect("target should resolve");
        assert_eq!(target, apple_simulator("iPhone 16"));
    }

    #[test]
    fn resolves_apple_host_target_from_explicit_host() {
        let target = resolve_selected_target(
            None,
            None,
            Some("192.0.2.1".to_string()),
            Some(12005),
            None,
            None,
        )
        .expect("target should resolve");
        assert_eq!(
            target,
            SelectedTarget::AppleHost {
                host: "192.0.2.1".to_string(),
                controlkit_port: Some(12005),
            }
        );
    }

    #[test]
    fn falls_back_to_stored_target_when_nothing_provided() {
        let stored = apple_simulator("iPad");
        let target = resolve_selected_target(None, None, None, None, None, Some(stored.clone()))
            .expect("target should resolve");
        assert_eq!(target, stored);
    }

    #[test]
    fn explicit_fields_override_stored_target() {
        let stored = apple_simulator("iPad");
        let target = resolve_selected_target(
            None,
            None,
            None,
            None,
            Some("emulator-5554".to_string()),
            Some(stored),
        )
        .expect("target should resolve");
        assert_eq!(
            target,
            SelectedTarget::Android {
                serial: Some("emulator-5554".to_string())
            }
        );
    }

    #[test]
    fn rejects_missing_target_with_no_stored_fallback() {
        let error = resolve_selected_target(None, None, None, None, None, None)
            .expect_err("target should not resolve");
        assert!(error.contains("No target provided"));
    }

    #[test]
    fn rejects_ambiguous_android_and_apple_simulator_fields() {
        let error = resolve_selected_target(
            Some("iPhone 16".to_string()),
            None,
            None,
            None,
            Some("emulator-5554".to_string()),
            None,
        )
        .expect_err("target should not resolve");
        assert!(error.contains("Ambiguous"));
    }

    #[test]
    fn rejects_ambiguous_android_and_apple_host_fields() {
        let error = resolve_selected_target(
            None,
            None,
            Some("192.0.2.1".to_string()),
            None,
            Some("emulator-5554".to_string()),
            None,
        )
        .expect_err("target should not resolve");
        assert!(error.contains("Ambiguous"));
    }

    #[test]
    fn rejects_ambiguous_simulator_and_host_fields() {
        let error = resolve_selected_target(
            Some("iPhone 16".to_string()),
            None,
            Some("192.0.2.1".to_string()),
            None,
            None,
            None,
        )
        .expect_err("target should not resolve");
        assert!(error.contains("Ambiguous"));
    }

    #[test]
    fn requires_apple_target_accepts_apple_simulator() {
        require_apple_target("ui_describe", &apple_simulator("iPhone 16"))
            .expect("apple simulator should be accepted");
    }

    #[test]
    fn requires_apple_target_accepts_apple_host() {
        require_apple_target(
            "ui_describe",
            &SelectedTarget::AppleHost {
                host: "192.0.2.1".to_string(),
                controlkit_port: None,
            },
        )
        .expect("apple host should be accepted");
    }

    #[test]
    fn requires_apple_target_rejects_android_with_tool_name_and_serial() {
        let error = require_apple_target(
            "ui_describe",
            &SelectedTarget::Android {
                serial: Some("emulator-5554".to_string()),
            },
        )
        .expect_err("android target should be rejected");
        assert!(error.contains("ui_describe"));
        assert!(error.contains("emulator-5554"));
    }

    #[test]
    fn converts_finite_coordinates_to_rounded_pixels() {
        assert_eq!(to_android_coordinate(12.4).unwrap(), 12);
        assert_eq!(to_android_coordinate(12.6).unwrap(), 13);
        // Rust's f64::round() rounds half away from zero.
        assert_eq!(to_android_coordinate(-3.5).unwrap(), -4);
    }

    #[test]
    fn rejects_non_finite_coordinates() {
        assert!(to_android_coordinate(f64::NAN).is_err());
        assert!(to_android_coordinate(f64::INFINITY).is_err());
    }

    #[test]
    fn parses_valid_apple_buttons_case_sensitively() {
        assert_eq!(AppleButton::parse("home"), Some(AppleButton::Home));
        assert_eq!(
            AppleButton::parse("playPause"),
            Some(AppleButton::PlayPause)
        );
        assert_eq!(AppleButton::parse("playpause"), None);
        assert_eq!(AppleButton::parse("back"), None);
    }

    #[test]
    fn parses_valid_android_buttons() {
        assert_eq!(AndroidButton::parse("back"), Some(AndroidButton::Back));
        assert_eq!(
            AndroidButton::parse("recents"),
            Some(AndroidButton::Recents)
        );
        assert_eq!(AndroidButton::parse("menu"), None);
    }

    #[test]
    fn orientation_as_str_matches_controlkit_vocabulary() {
        assert_eq!(Orientation::Portrait.as_str(), "PORTRAIT");
        assert_eq!(Orientation::Landscape.as_str(), "LANDSCAPE");
    }

    #[test]
    fn android_capabilities_lists_supported_and_unsupported_actions() {
        let capabilities = android_capabilities();
        assert_eq!(capabilities["screen_capture"], serde_json::json!(true));
        assert_eq!(capabilities["ui_describe"], serde_json::json!(false));
        assert_eq!(
            capabilities["input_button"]["buttons"],
            serde_json::json!(AndroidButton::ALL)
        );
    }
}
