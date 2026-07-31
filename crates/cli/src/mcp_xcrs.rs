//! Automation MCP profile: `smb --mcp --scope automation` re-exposes the
//! canonical cross-platform mobile and TV automation tool set defined in the
//! `xcrs` crate, under its own [`AutomationMcpServer`].
//!
//! This is a separate server type rather than more tools bolted onto
//! [`super::SmbMcpServer`] so a single MCP session only ever sees one
//! profile's tools — cloud or automation, never both merged together.

use {
    anyhow::{anyhow, Result},
    rmcp::{
        model::{Implementation, ServerCapabilities, ServerInfo},
        transport::stdio,
        ServerHandler, ServiceExt,
    },
};

/// The shared XCRS mobile/TV automation MCP server. Tool names carry no
/// `smb_`/`xcrs_` prefix: the MCP client already namespaces tools by server,
/// so a prefix here would just duplicate that.
#[derive(Debug, Default)]
pub struct AutomationMcpServer;

impl AutomationMcpServer {
    pub fn new() -> Self {
        Self
    }
}

xcrs::xcrs_mcp_tools!(
    AutomationMcpServer,
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
impl ServerHandler for AutomationMcpServer {
    fn get_info(&self) -> ServerInfo {
        // `Implementation` is `#[non_exhaustive]`, so start from the build-env
        // default and override the identity fields.
        let mut server_info = Implementation::from_build_env();
        server_info.name = "XCRS Mobile & TV Automation".to_string();
        server_info.version = env!("CARGO_PKG_VERSION").to_string();

        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_info)
            .with_instructions(
                "Cross-platform mobile and TV app automation over Apple simulators/ControlKit \
                 and Android adb. Start with `device_list` to discover available simulators and \
                 adb-connected devices, then `device_select` to remember one for later calls, \
                 then `device_capabilities` to check what it supports before choosing an \
                 inspection or input tool. Every action tool also accepts an inline target \
                 instead of the remembered one. Use plain `smb --mcp` instead for smbCloud \
                 account, project, tenant, Mail, and Auth tools.",
            )
    }
}

/// Run the automation MCP server over stdio until the client disconnects.
pub async fn serve() -> Result<()> {
    let running = AutomationMcpServer::new()
        .serve(stdio())
        .await
        .map_err(|error| anyhow!("Failed to start automation MCP server: {error}"))?;
    running
        .waiting()
        .await
        .map_err(|error| anyhow!("Automation MCP server stopped unexpectedly: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn automation_router_exposes_exactly_the_canonical_eighteen_tools() {
        let tools = AutomationMcpServer::xcrs_tool_router().list_all();
        let mut names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
        names.sort_unstable();

        let mut expected = CANONICAL_TOOL_NAMES;
        expected.sort_unstable();

        assert_eq!(names, expected, "automation router tool set changed");
    }

    #[test]
    fn automation_tool_names_carry_no_server_prefix() {
        let tools = AutomationMcpServer::xcrs_tool_router().list_all();

        for tool in &tools {
            assert!(
                !tool.name.starts_with("smb_") && !tool.name.starts_with("xcrs_"),
                "tool name {:?} should not carry a redundant server prefix",
                tool.name
            );
        }
    }

    #[test]
    fn automation_router_excludes_cloud_tools() {
        let tools = AutomationMcpServer::xcrs_tool_router().list_all();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();

        for cloud_only in [
            "me",
            "project_list",
            "tenant_list",
            "mail_list",
            "auth_app_list",
        ] {
            assert!(
                !names.contains(&cloud_only),
                "automation router should not expose cloud tool {cloud_only:?}"
            );
        }
    }
}
