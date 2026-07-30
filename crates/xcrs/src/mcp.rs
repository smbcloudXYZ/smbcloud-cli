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

#[derive(Debug, Default)]
pub struct XcrsMcpServer;

impl XcrsMcpServer {
    pub fn new() -> Self {
        Self
    }
}

#[macro_export]
macro_rules! xcrs_mcp_tools {
    ($server:ty, $simulator_list_name:literal, $simulator_find_name:literal, $ios_app_test_name:literal) => {
        #[::rmcp::tool_router(router = xcrs_tool_router, vis = "pub(crate)")]
        impl $server {
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
        }
    };
}

xcrs_mcp_tools!(
    XcrsMcpServer,
    "xcrs_simulator_list",
    "xcrs_simulator_find",
    "xcrs_ios_app_test"
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
