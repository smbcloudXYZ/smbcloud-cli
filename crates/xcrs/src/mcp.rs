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
    /// Local ControlKit JSON-RPC port. Defaults to 12004.
    #[serde(default)]
    pub controlkit_port: Option<u16>,
    /// tvOS button: up, down, left, right, select, menu, home, or playPause.
    pub button: String,
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
        $simulator_home_name:literal,
        $simulator_button_name:literal,
        $simulator_orientation_get_name:literal,
        $simulator_orientation_set_name:literal
    ) => {
        #[::rmcp::tool_router(router = xcrs_tool_router, vis = "pub(crate)")]
        impl $server {
            fn simulator_from_target(
                target: &$crate::mcp::SimulatorTargetArgs,
            ) -> ::std::result::Result<$crate::Simulator, ::rmcp::model::ErrorData> {
                let tools = $crate::XcodeCommandLineTools::new();
                match (&target.simulator_udid, &target.simulator_name) {
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
                        "Provide either simulator_name or simulator_udid.",
                        None,
                    )),
                }
            }

            fn controlkit_from_target(
                simulator_name: &Option<String>,
                simulator_udid: &Option<String>,
                controlkit_port: Option<u16>,
            ) -> ::std::result::Result<
                ($crate::Simulator, $crate::ControlKit),
                ::rmcp::model::ErrorData,
            > {
                let target = $crate::mcp::SimulatorTargetArgs {
                    simulator_name: simulator_name.clone(),
                    simulator_udid: simulator_udid.clone(),
                    controlkit_port,
                };
                let simulator = Self::simulator_from_target(&target)?;
                Ok((
                    simulator,
                    $crate::ControlKit::new(controlkit_port.unwrap_or(12004)),
                ))
            }

            #[::rmcp::tool(
                name = $simulator_list_name,
                description = "List all iOS simulators known to Xcode, including their runtime, UDID, availability, and current state."
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
                description = "Find an iOS simulator by its exact name and return its details as JSON."
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
                description = "Boot an iOS simulator, install an .app bundle, optionally terminate the bundle, launch it, and return its app container path."
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
                description = "Capture a PNG screenshot from an iOS simulator."
            )]
            async fn simulator_screenshot(
                &self,
                ::rmcp::handler::server::wrapper::Parameters(
                    args,
                ): ::rmcp::handler::server::wrapper::Parameters<
                    $crate::mcp::SimulatorTargetArgs,
                >,
            ) -> ::std::result::Result<
                ::rmcp::model::CallToolResult,
                ::rmcp::model::ErrorData,
            > {
                let simulator = Self::simulator_from_target(&args)?;
                let screenshot = $crate::XcodeCommandLineTools::new()
                    .simctl()
                    .screenshot(&simulator.udid)
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                let encoded = $crate::encode_base64(screenshot);
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::image(encoded, "image/png"),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_launch_app_name,
                description = "Launch an installed app on an iOS simulator by bundle identifier."
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
                let target = $crate::mcp::SimulatorTargetArgs {
                    simulator_name: args.simulator_name,
                    simulator_udid: args.simulator_udid,
                    controlkit_port: args.controlkit_port,
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
                description = "Terminate an installed app on an iOS simulator by bundle identifier."
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
                let target = $crate::mcp::SimulatorTargetArgs {
                    simulator_name: args.simulator_name,
                    simulator_udid: args.simulator_udid,
                    controlkit_port: args.controlkit_port,
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
                description = "Open a URL or custom URL scheme on an iOS simulator."
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
                description = "Return the accessibility UI hierarchy from the foreground iOS app through ControlKit."
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
                description = "List actionable accessibility elements and coordinates from the foreground iOS app through ControlKit."
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
                description = "Tap the iOS simulator screen at the given coordinates through ControlKit."
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
                        args.x, args.y, simulator.name
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_text_name,
                description = "Type text into the focused iOS simulator field through ControlKit."
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
                        simulator.name
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_swipe_name,
                description = "Swipe between two screen coordinates on an iOS simulator through ControlKit."
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
                        simulator.name
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_home_name,
                description = "Press the iOS simulator Home button through ControlKit."
            )]
            async fn simulator_home(
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
                    args.controlkit_port,
                )?;
                controlkit
                    .call("device.io.button", ::serde_json::json!({ "button": "home" }))
                    .await
                    .map_err(|error| {
                        ::rmcp::model::ErrorData::internal_error(error.to_string(), None)
                    })?;
                Ok(::rmcp::model::CallToolResult::success(vec![
                    ::rmcp::model::ContentBlock::text(format!(
                        "Pressed Home on {}.",
                        simulator.name
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_button_name,
                description = "Press a ControlKit remote button on an iOS or tvOS simulator. tvOS supports up, down, left, right, select, menu, home, and playPause."
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
                        args.button, simulator.name
                    )),
                ]))
            }

            #[::rmcp::tool(
                name = $simulator_orientation_get_name,
                description = "Read the current iOS simulator orientation through ControlKit."
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
                description = "Set the iOS simulator orientation to PORTRAIT or LANDSCAPE through ControlKit."
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
                        orientation, simulator.name
                    )),
                ]))
            }
        }
    };
}

xcrs_mcp_tools!(
    XcrsMcpServer,
    "xcrs_simulator_list",
    "xcrs_simulator_find",
    "xcrs_ios_app_test",
    "xcrs_simulator_screenshot",
    "xcrs_simulator_launch_app",
    "xcrs_simulator_terminate_app",
    "xcrs_simulator_open_url",
    "xcrs_simulator_ui_dump",
    "xcrs_simulator_list_elements",
    "xcrs_simulator_tap",
    "xcrs_simulator_type_text",
    "xcrs_simulator_swipe",
    "xcrs_simulator_press_home",
    "xcrs_simulator_press_button",
    "xcrs_simulator_orientation_get",
    "xcrs_simulator_orientation_set"
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
                "xcrs exposes Xcode command line tools for iOS simulators and app testing.",
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
