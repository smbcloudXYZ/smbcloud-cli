use {
    anyhow::{anyhow, Result},
    rmcp::{
        model::{Implementation, ServerCapabilities, ServerInfo},
        transport::stdio,
        ServerHandler, ServiceExt,
    },
    schemars::JsonSchema,
    serde::Deserialize,
    std::path::PathBuf,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorFindArgs {
    /// Exact simulator name to find.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScreenshotArgs {
    /// Simulator name of the target to capture. Omit to use the target selected
    /// with the use-target tool.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID of the target to capture.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// Physical device identifier known to CoreDevice (UDID, ECID, serial
    /// number, user-provided name, or DNS name). Provide this to capture a
    /// paired physical device instead of a simulator.
    #[serde(default)]
    pub device: Option<String>,
}

/// A target chosen once with the use-target tool and reused by later actions so
/// they no longer need per-call target arguments.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema, serde::Serialize)]
pub struct StoredTarget {
    #[serde(default)]
    pub simulator_name: Option<String>,
    #[serde(default)]
    pub simulator_udid: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub controlkit_port: Option<u16>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UseTargetArgs {
    /// Exact simulator name to remember as the active target.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID to remember as the active target.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host of a physical device or remote runner to remember.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port to remember. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IosAppTestArgs {
    /// Exact simulator name to use.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID to use.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// Path to the `.app` bundle.
    pub app_path: PathBuf,
    /// Installed app bundle identifier.
    pub bundle_id: String,
    /// Terminate the app before launching it.
    #[serde(default)]
    pub terminate_before_launch: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorTargetArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorAppArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host for a physical device or remote runner. Omit for a local simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// App bundle identifier.
    pub bundle_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorOpenUrlArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// HTTP(S) URL or custom URL scheme.
    pub url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorTapArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host for a physical device or remote runner. Omit for a local simulator.
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorTextArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host for a physical device or remote runner. Omit for a local simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Text to type into the focused field.
    pub text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorSwipeArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host for a physical device or remote runner. Omit for a local simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Swipe start horizontal coordinate.
    pub x1: i32,
    /// Swipe start vertical coordinate.
    pub y1: i32,
    /// Swipe end horizontal coordinate.
    pub x2: i32,
    /// Swipe end vertical coordinate.
    pub y2: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorOrientationArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host for a physical device or remote runner. Omit for a local simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// Desired orientation: PORTRAIT or LANDSCAPE.
    pub orientation: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorControlKitArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host for a physical device or remote runner. Omit for a local simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorButtonArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host for a physical device or remote runner. Omit for a local simulator.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// tvOS button: up, down, left, right, select, menu, home, or playPause.
    pub button: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ControlKitEndpointArgs {
    /// Exact simulator name. Omit this when connecting to a local macOS runner.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID. Omit this when connecting to a local macOS runner.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host. Defaults to 127.0.0.1 for local runners.
    #[serde(default)]
    pub host: Option<String>,
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ControlKitCoordinateArgs {
    /// Exact simulator name. Omit this when connecting to a local macOS runner.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID. Omit this when connecting to a local macOS runner.
    #[serde(default)]
    pub simulator_udid: Option<String>,
    /// ControlKit host. Defaults to 127.0.0.1 for local runners.
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
        $simulator_list_name:literal,
        $simulator_find_name:literal,
        $ios_app_test_name:literal,
        $simulator_screenshot_name:literal,
        $simulator_launch_app_name:literal,
        $simulator_terminate_app_name:literal,
        $simulator_open_url_name:literal,
        $simulator_ui_dump_name:literal,
        $simulator_list_elements_name:literal,
        $simulator_tap_name:literal,
        $simulator_text_name:literal,
        $simulator_swipe_name:literal,
        $simulator_button_name:literal,
        $simulator_orientation_get_name:literal,
        $simulator_orientation_set_name:literal,
        $controlkit_capabilities_name:literal,
        $macos_click_name:literal,
        $visionos_spatial_tap_name:literal,
        $use_target_name:literal
    ) => {
        #[::rmcp::tool_router(router = xcrs_tool_router, vis = "pub(crate)")]
        impl $server {
            fn selected_target_slot(
            ) -> &'static ::std::sync::Mutex<Option<$crate::mcp::StoredTarget>> {
                static SLOT: ::std::sync::Mutex<Option<$crate::mcp::StoredTarget>> =
                    ::std::sync::Mutex::new(None);
                &SLOT
            }

            fn stored_target() -> Option<$crate::mcp::StoredTarget> {
                Self::selected_target_slot()
                    .lock()
                    .ok()
                    .and_then(|slot| slot.clone())
            }

            /// Fill missing target arguments from the target selected with the
            /// use-target tool so per-call target fields become optional.
            fn resolve_target_fields(
                simulator_name: &Option<String>,
                simulator_udid: &Option<String>,
                host: &Option<String>,
                controlkit_port: Option<u16>,
            ) -> (Option<String>, Option<String>, Option<String>, Option<u16>) {
                if simulator_name.is_none() && simulator_udid.is_none() && host.is_none() {
                    if let Some(stored) = Self::stored_target() {
                        return (
                            stored.simulator_name,
                            stored.simulator_udid,
                            stored.host,
                            controlkit_port.or(stored.controlkit_port),
                        );
                    }
                }
                (
                    simulator_name.clone(),
                    simulator_udid.clone(),
                    host.clone(),
                    controlkit_port,
                )
            }

            fn simulator_from_target(
                target: &$crate::mcp::SimulatorTargetArgs,
            ) -> ::std::result::Result<$crate::Simulator, ::rmcp::model::ErrorData> {
                let tools = $crate::XcodeCommandLineTools::new();
                let (simulator_name, simulator_udid, _host, _port) = Self::resolve_target_fields(
                    &target.simulator_name,
                    &target.simulator_udid,
                    &None,
                    target.controlkit_port,
                );
                match (&simulator_udid, &simulator_name) {
                    (Some(udid), _) => tools
                        .simctl()
                        .find_simulator_by_udid(udid)
                        .map_err(|error| {
                            ::rmcp::model::ErrorData::invalid_request(error.to_string(), None)
                        }),
                    (None, Some(name)) => tools
                        .simctl()
                        .find_simulator_by_name(name)
                        .map_err(|error| {
                            ::rmcp::model::ErrorData::invalid_request(error.to_string(), None)
                        }),
                    (None, None) => Err(::rmcp::model::ErrorData::invalid_request(
                        "No target. Pass simulator_name or simulator_udid, or select one first with the use-target tool.",
                        None,
                    )),
                }
            }

            fn controlkit_from_target(
                simulator_name: &Option<String>,
                simulator_udid: &Option<String>,
                host: &Option<String>,
                controlkit_port: Option<u16>,
            ) -> ::std::result::Result<
                (Option<$crate::Simulator>, $crate::ControlKit),
                ::rmcp::model::ErrorData,
            > {
                let (simulator_name, simulator_udid, host, controlkit_port) =
                    Self::resolve_target_fields(
                        simulator_name,
                        simulator_udid,
                        host,
                        controlkit_port,
                    );
                if let Some(host) = host {
                    return Ok((
                        None,
                        $crate::ControlKit::with_host(&host, controlkit_port.unwrap_or(12004)),
                    ));
                }
                let target = $crate::mcp::SimulatorTargetArgs {
                    simulator_name,
                    simulator_udid,
                    controlkit_port,
                };
                let simulator = Self::simulator_from_target(&target)?;
                Ok((
                    Some(simulator),
                    $crate::ControlKit::new(controlkit_port.unwrap_or(12004)),
                ))
            }

            fn target_label(simulator: &Option<$crate::Simulator>, host: &Option<String>) -> String {
                if let Some(simulator) = simulator {
                    return simulator.name.clone();
                }
                host.clone().unwrap_or_else(|| "127.0.0.1".to_string())
            }

            fn controlkit_from_endpoint(
                endpoint: &$crate::mcp::ControlKitEndpointArgs,
            ) -> ::std::result::Result<
                ($crate::ControlKit, Option<$crate::Simulator>),
                ::rmcp::model::ErrorData,
            > {
                let (simulator_name, simulator_udid, host, controlkit_port) =
                    Self::resolve_target_fields(
                        &endpoint.simulator_name,
                        &endpoint.simulator_udid,
                        &endpoint.host,
                        endpoint.controlkit_port,
                    );
                let simulator = match (&simulator_udid, &simulator_name) {
                    (Some(udid), _) => Some(
                        $crate::XcodeCommandLineTools::new()
                            .simctl()
                            .find_simulator_by_udid(udid)
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::invalid_request(
                                    error.to_string(),
                                    None,
                                )
                            })?,
                    ),
                    (None, Some(name)) => Some(
                        $crate::XcodeCommandLineTools::new()
                            .simctl()
                            .find_simulator_by_name(name)
                            .map_err(|error| {
                                ::rmcp::model::ErrorData::invalid_request(
                                    error.to_string(),
                                    None,
                                )
                            })?,
                    ),
                    (None, None) => None,
                };
                let host = host.unwrap_or_else(|| "127.0.0.1".to_string());
                Ok((
                    $crate::ControlKit::with_host(
                        &host,
                        controlkit_port.unwrap_or(12004),
                    ),
                    simulator,
                ))
            }

            fn endpoint_from_coordinate(
                args: &$crate::mcp::ControlKitCoordinateArgs,
            ) -> $crate::mcp::ControlKitEndpointArgs {
                $crate::mcp::ControlKitEndpointArgs {
                    simulator_name: args.simulator_name.clone(),
                    simulator_udid: args.simulator_udid.clone(),
                    host: args.host.clone(),
                    controlkit_port: args.controlkit_port,
                }
            }

            #[::rmcp::tool(
                name = $use_target_name,
                description = "Select the Apple target (simulator or physical/remote device) that later UI actions use by default, so you don't repeat target arguments on every call. Pass a simulator_name/simulator_udid for a simulator, or a host (plus optional controlkit_port) for a physical device or remote runner. Call this once after picking a target with the list-simulators tool; individual actions can still override it with their own target arguments."
            )]
            async fn use_target(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::UseTargetArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                if args.simulator_name.is_none()
                    && args.simulator_udid.is_none()
                    && args.host.is_none()
                {
                    return Err(::rmcp::model::ErrorData::invalid_request(
                        "Provide a simulator_name/simulator_udid or a host to select as the active target.",
                        None,
                    ));
                }
                let stored = $crate::mcp::StoredTarget {
                    simulator_name: args.simulator_name,
                    simulator_udid: args.simulator_udid,
                    host: args.host,
                    controlkit_port: args.controlkit_port,
                };
                match Self::selected_target_slot().lock() {
                    Ok(mut slot) => *slot = Some(stored.clone()),
                    Err(_) => {
                        return Err(::rmcp::model::ErrorData::internal_error(
                            "Failed to store the selected target.",
                            None,
                        ))
                    }
                }
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&::serde_json::json!({
                        "selected": stored,
                    }))?,
                ]))
            }

            #[::rmcp::tool(
                name = $controlkit_capabilities_name,
                description = "Report the platform and supported input capabilities of the active Apple target. Use this to discover whether the target supports taps, spatial taps, buttons, or orientation before driving it. Works on a simulator, a physical/remote device, or the local Mac."
            )]
            async fn controlkit_capabilities(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::ControlKitEndpointArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (controlkit, simulator) = Self::controlkit_from_endpoint(&args)?;
                let result = controlkit
                    .call("device.capabilities", ::serde_json::json!({}))
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&::serde_json::json!({
                        "simulator": simulator,
                        "capabilities": result,
                    }))?,
                ]))
            }

            #[::rmcp::tool(
                name = $macos_click_name,
                description = "Click the running macOS app at screen coordinates. Use this for macOS targets; use the tap tool for iOS/tvOS/watchOS and the gesture tool for visionOS."
            )]
            async fn macos_click(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::ControlKitCoordinateArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let endpoint = Self::endpoint_from_coordinate(&args);
                let (controlkit, _) = Self::controlkit_from_endpoint(&endpoint)?;
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
                name = $visionos_spatial_tap_name,
                description = "Perform a spatial tap on the running visionOS app at screen coordinates. Use this for visionOS targets; use the tap tool for iOS/tvOS/watchOS and the click tool for macOS."
            )]
            async fn visionos_spatial_tap(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::ControlKitCoordinateArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let endpoint = Self::endpoint_from_coordinate(&args);
                let (controlkit, _) = Self::controlkit_from_endpoint(&endpoint)?;
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
                name = $simulator_list_name,
                description = "List every Apple simulator known to Xcode with its platform, runtime, UDID, availability, and current state. Start here to pick a simulator, then remember it with the use-target tool."
            )]
            async fn simulator_list(
                &self,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let simulators = $crate::XcodeCommandLineTools::new()
                    .simctl()
                    .list_simulators()
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&simulators)?,
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_find_name,
                description = "Find an Apple simulator by its exact name and return its details as JSON. Use this to resolve a UDID before selecting it with the use-target tool."
            )]
            async fn simulator_find(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorFindArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let simulator = $crate::XcodeCommandLineTools::new()
                    .simctl()
                    .find_simulator_by_name(&args.name)
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::json(&simulator)?,
                ]))
            }

            #[::rmcp::tool(
                name = $ios_app_test_name,
                description = "Boot an iOS simulator, install an .app bundle, optionally terminate it first, launch it, and return its app container path. Simulator-only end-to-end setup step; use launch-app/terminate-app for an already-installed app."
            )]
            async fn ios_app_test(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::IosAppTestArgs,
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
                name = $simulator_screenshot_name,
                description = "Capture a PNG screenshot of the running app. Targets a simulator (via simulator_name/simulator_udid or the selected target) or a paired physical device (via the device identifier). Use describe-ui instead when you need element coordinates rather than pixels."
            )]
            async fn simulator_screenshot(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::ScreenshotArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let screenshot = if let Some(device) = &args.device {
                    $crate::XcodeCommandLineTools::new()
                        .devicectl()
                        .screenshot(device)
                        .map_err(|error| {
                            ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                        })?
                } else {
                    let target = $crate::mcp::SimulatorTargetArgs {
                        simulator_name: args.simulator_name.clone(),
                        simulator_udid: args.simulator_udid.clone(),
                        controlkit_port: None,
                    };
                    let simulator = Self::simulator_from_target(&target)?;
                    $crate::XcodeCommandLineTools::new()
                        .simctl()
                        .screenshot(&simulator.udid)
                        .map_err(|error| {
                            ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                        })?
                };
                let encoded = $crate::encode_base64(screenshot);
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::image(encoded, "image/png"),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_launch_app_name,
                description = "Launch an installed app by bundle identifier on the active target. Works on a simulator or a physical/remote device."
            )]
            async fn simulator_launch_app(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorAppArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator_name, simulator_udid, host, controlkit_port) =
                    Self::resolve_target_fields(
                        &args.simulator_name,
                        &args.simulator_udid,
                        &args.host,
                        args.controlkit_port,
                    );
                if let Some(host) = &host {
                    let controlkit =
                        $crate::ControlKit::with_host(host, controlkit_port.unwrap_or(12004));
                    controlkit
                        .call(
                            "device.apps.launch",
                            ::serde_json::json!({ "bundleId": args.bundle_id }),
                        )
                        .await
                        .map_err(|error| {
                            ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                        })?;
                    return Ok(::rmcp::model::CallToolResult::success(vec![
                        ::rmcp::model::ContentBlock::text(format!(
                            "Launched {} on {}.",
                            args.bundle_id, host
                        )),
                    ]));
                }
                let target = $crate::mcp::SimulatorTargetArgs {
                    simulator_name,
                    simulator_udid,
                    controlkit_port,
                };
                let simulator = Self::simulator_from_target(&target)?;
                $crate::XcodeCommandLineTools::new()
                    .simctl()
                    .launch_app(&simulator.udid, &args.bundle_id)
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Launched {} on {}.",
                        args.bundle_id, simulator.name
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_terminate_app_name,
                description = "Terminate an installed app by bundle identifier on the active target. Works on a simulator or a physical/remote device."
            )]
            async fn simulator_terminate_app(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorAppArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator_name, simulator_udid, host, controlkit_port) =
                    Self::resolve_target_fields(
                        &args.simulator_name,
                        &args.simulator_udid,
                        &args.host,
                        args.controlkit_port,
                    );
                if let Some(host) = &host {
                    let controlkit =
                        $crate::ControlKit::with_host(host, controlkit_port.unwrap_or(12004));
                    controlkit
                        .call(
                            "device.apps.terminate",
                            ::serde_json::json!({ "bundleId": args.bundle_id }),
                        )
                        .await
                        .map_err(|error| {
                            ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                        })?;
                    return Ok(::rmcp::model::CallToolResult::success(vec![
                        ::rmcp::model::ContentBlock::text(format!(
                            "Terminated {} on {}.",
                            args.bundle_id, host
                        )),
                    ]));
                }
                let target = $crate::mcp::SimulatorTargetArgs {
                    simulator_name,
                    simulator_udid,
                    controlkit_port,
                };
                let simulator = Self::simulator_from_target(&target)?;
                $crate::XcodeCommandLineTools::new()
                    .simctl()
                    .terminate_app(&simulator.udid, &args.bundle_id)
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Terminated {} on {}.",
                        args.bundle_id, simulator.name
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_open_url_name,
                description = "Open a URL or custom URL scheme on the active simulator target (simulator only)."
            )]
            async fn simulator_open_url(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorOpenUrlArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let target = $crate::mcp::SimulatorTargetArgs {
                    simulator_name: args.simulator_name,
                    simulator_udid: args.simulator_udid,
                    controlkit_port: args.controlkit_port,
                };
                let simulator = Self::simulator_from_target(&target)?;
                $crate::XcodeCommandLineTools::new()
                    .simctl()
                    .open_url(&simulator.udid, &args.url)
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Opened {} on {}.",
                        args.url, simulator.name
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_ui_dump_name,
                description = "Return the full accessibility hierarchy of the foreground app as JSON. Use this first to see what is on screen before tapping or typing. Works on a simulator or a physical/remote device. Prefer list-elements when you only need actionable elements and their tap coordinates."
            )]
            async fn simulator_ui_dump(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorControlKitArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator, controlkit) = Self::controlkit_from_target(
                    &args.simulator_name,
                    &args.simulator_udid,
                    &args.host,
                    args.controlkit_port,
                )?;
                let result = controlkit
                    .call("device.dump.ui", ::serde_json::json!({ "format": "json" }))
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
                name = $simulator_list_elements_name,
                description = "List just the actionable accessibility elements of the foreground app with their labels and tap coordinates. Use this to decide where to tap. Works on a simulator or a physical/remote device. Use describe-ui when you need the full hierarchy."
            )]
            async fn simulator_list_elements(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorControlKitArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator, controlkit) = Self::controlkit_from_target(
                    &args.simulator_name,
                    &args.simulator_udid,
                    &args.host,
                    args.controlkit_port,
                )?;
                let ui = controlkit
                    .call("device.dump.ui", ::serde_json::json!({ "format": "json" }))
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
                name = $simulator_tap_name,
                description = "Tap the running app at screen coordinates. Use this for iOS, tvOS, and watchOS targets, on a simulator or a physical/remote device. Use the click tool for macOS and the gesture tool for visionOS. Read coordinates from list-elements or describe-ui first."
            )]
            async fn simulator_tap(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorTapArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator, controlkit) = Self::controlkit_from_target(
                    &args.simulator_name,
                    &args.simulator_udid,
                    &args.host,
                    args.controlkit_port,
                )?;
                controlkit
                    .call(
                        "device.io.tap",
                        ::serde_json::json!({ "x": args.x, "y": args.y }),
                    )
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Tapped ({}, {}) on {}.",
                        args.x, args.y, Self::target_label(&simulator, &args.host)
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_text_name,
                description = "Type text into the focused field of the running app. Works on a simulator or a physical/remote device. Tap the field first to focus it."
            )]
            async fn simulator_text(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorTextArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator, controlkit) = Self::controlkit_from_target(
                    &args.simulator_name,
                    &args.simulator_udid,
                    &args.host,
                    args.controlkit_port,
                )?;
                controlkit
                    .call("device.io.text", ::serde_json::json!({ "text": args.text }))
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Typed text on {}.",
                        Self::target_label(&simulator, &args.host)
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_swipe_name,
                description = "Swipe between two screen coordinates in the running app, e.g. to scroll. Works on a simulator or a physical/remote device."
            )]
            async fn simulator_swipe(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorSwipeArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator, controlkit) = Self::controlkit_from_target(
                    &args.simulator_name,
                    &args.simulator_udid,
                    &args.host,
                    args.controlkit_port,
                )?;
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
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Swiped on {}.",
                        Self::target_label(&simulator, &args.host)
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_button_name,
                description = "Press a hardware or remote button on the running target: home (iOS), or up, down, left, right, select, menu, playPause (tvOS remote). Works on a simulator or a physical/remote device. Use this to press Home instead of a dedicated tool."
            )]
            async fn simulator_button(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorButtonArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator, controlkit) = Self::controlkit_from_target(
                    &args.simulator_name,
                    &args.simulator_udid,
                    &args.host,
                    args.controlkit_port,
                )?;
                let supported_buttons = [
                    "up",
                    "down",
                    "left",
                    "right",
                    "select",
                    "menu",
                    "home",
                    "playPause",
                ];
                if !supported_buttons.contains(&args.button.as_str()) {
                    return Err(::rmcp::model::ErrorData::invalid_request(
                        "button must be one of up, down, left, right, select, menu, home, or playPause",
                        None,
                    ));
                }
                controlkit
                    .call(
                        "device.io.button",
                        ::serde_json::json!({ "button": args.button }),
                    )
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Pressed {} on {}.",
                        args.button, Self::target_label(&simulator, &args.host)
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_orientation_get_name,
                description = "Read the current screen orientation of the running target. Works on a simulator or a physical/remote device."
            )]
            async fn simulator_orientation_get(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorControlKitArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator, controlkit) = Self::controlkit_from_target(
                    &args.simulator_name,
                    &args.simulator_udid,
                    &args.host,
                    args.controlkit_port,
                )?;
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
                name = $simulator_orientation_set_name,
                description = "Set the screen orientation of the running target to PORTRAIT or LANDSCAPE. Works on a simulator or a physical/remote device."
            )]
            async fn simulator_orientation_set(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorOrientationArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let (simulator, controlkit) = Self::controlkit_from_target(
                    &args.simulator_name,
                    &args.simulator_udid,
                    &args.host,
                    args.controlkit_port,
                )?;
                let orientation = args.orientation.to_uppercase();
                if orientation != "PORTRAIT" && orientation != "LANDSCAPE" {
                    return Err(::rmcp::model::ErrorData::invalid_request(
                        "orientation must be PORTRAIT or LANDSCAPE",
                        None,
                    ));
                }
                controlkit
                    .call(
                        "device.io.orientation.set",
                        ::serde_json::json!({ "orientation": orientation }),
                    )
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Set orientation to {} on {}.",
                        orientation, Self::target_label(&simulator, &args.host)
                    )),
                ]))
            }
        }
    };
}

xcrs_mcp_tools!(
    XcrsMcpServer,
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
    "xcrs_orientation_get",
    "xcrs_orientation_set",
    "xcrs_capabilities",
    "xcrs_click",
    "xcrs_gesture",
    "xcrs_use_target"
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
                "xcrs exposes Xcode command line tools and ControlKit runners for Apple platform testing.",
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
