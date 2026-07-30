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
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorAppArgs {
    /// Exact simulator name.
    #[serde(default)]
    pub simulator_name: Option<String>,
    /// Simulator UDID.
    #[serde(default)]
    pub simulator_udid: Option<String>,
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
    /// HTTP(S) URL or custom URL scheme.
    pub url: String,
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
        $simulator_open_url_name:literal
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
    "xcrs_simulator_open_url"
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
