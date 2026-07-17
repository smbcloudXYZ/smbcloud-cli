//! MCP (Model Context Protocol) server interface.
//!
//! When `smb` is started with `--mcp`, it runs as an MCP server over stdio
//! instead of executing a one-shot command. The server exposes smbCloud
//! operations as MCP tools built on the official `rmcp` SDK.
//!
//! Tools call the same library functions the CLI handlers use, but return
//! structured JSON rather than rendering spinners or a TUI — the stdout stream
//! is the JSON-RPC channel and must stay free of console output. Logging still
//! goes to the on-disk log file (set up by `main`), never to stdout.

use {
    crate::{account::lib::is_logged_in, client, token::get_smb_token::get_smb_token},
    anyhow::{anyhow, Result},
    rmcp::{
        handler::server::ServerHandler,
        model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo},
        tool, tool_handler, tool_router,
        transport::stdio,
        ErrorData, ServiceExt,
    },
    smbcloud_auth::me::me,
    smbcloud_network::environment::Environment,
};

/// The smbCloud MCP server. Holds the environment selected on the command line
/// so every tool talks to the same API host and on-disk state dir.
pub struct SmbMcpServer {
    environment: Environment,
}

impl SmbMcpServer {
    pub fn new(environment: Environment) -> Self {
        Self { environment }
    }
}

#[tool_router]
impl SmbMcpServer {
    #[tool(description = "Get the authenticated smbCloud user's account info. \
                          Requires a prior `smb login`; returns the user as JSON.")]
    async fn me(&self) -> Result<CallToolResult, ErrorData> {
        if !is_logged_in(self.environment) {
            return Err(ErrorData::invalid_request(
                "Not logged in. Run `smb login` first.",
                None,
            ));
        }
        let token = get_smb_token(self.environment)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let user = me(self.environment, client(), &token)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let content = ContentBlock::json(&user)?;
        Ok(CallToolResult::success(vec![content]))
    }
}

#[tool_handler]
impl ServerHandler for SmbMcpServer {
    fn get_info(&self) -> ServerInfo {
        // `Implementation` is `#[non_exhaustive]`, so start from the build-env
        // default and override the identity fields to report `smb`, not `rmcp`.
        let mut server_info = Implementation::from_build_env();
        server_info.name = "smb".to_string();
        server_info.version = env!("CARGO_PKG_VERSION").to_string();

        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_info)
            .with_instructions(
                "smbCloud CLI exposed as MCP tools. Authentication uses the token stored by \
                 `smb login`; tools run non-interactively.",
            )
    }
}

/// Run the MCP server over stdio until the client disconnects.
pub async fn serve(environment: Environment) -> Result<()> {
    let running = SmbMcpServer::new(environment)
        .serve(stdio())
        .await
        .map_err(|e| anyhow!("Failed to start MCP server: {e}"))?;
    running
        .waiting()
        .await
        .map_err(|e| anyhow!("MCP server stopped unexpectedly: {e}"))?;
    Ok(())
}
