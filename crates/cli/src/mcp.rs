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
        handler::server::{wrapper::Parameters, ServerHandler},
        model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo},
        tool, tool_handler, tool_router,
        transport::stdio,
        ErrorData, ServiceExt,
    },
    schemars::JsonSchema,
    serde::Deserialize,
    smbcloud_auth::me::me,
    smbcloud_model::project::ProjectCreate,
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::{
        crud_project_create::create_project, crud_project_delete::delete_project,
        crud_project_deployment_read::get_deployments, crud_project_read::get_project,
        crud_project_read::get_projects, crud_project_update::update_project,
    },
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

    /// Resolve the stored auth token, mapping "not logged in" and read failures
    /// to MCP errors. Every tool that hits the API goes through this.
    fn access_token(&self) -> Result<String, ErrorData> {
        if !is_logged_in(self.environment) {
            return Err(ErrorData::invalid_request(
                "Not logged in. Run `smb login` first.",
                None,
            ));
        }
        get_smb_token(self.environment).map_err(|e| ErrorData::internal_error(e.to_string(), None))
    }
}

/// Serialize any JSON-able value into a single-content successful tool result.
fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::json(value)?]))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProjectShowArgs {
    /// The project ID to show.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeploymentsArgs {
    /// The project ID whose deployments to list.
    project_id: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProjectCreateArgs {
    /// Name for the new project.
    name: String,
    /// Optional description for the new project.
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProjectUpdateArgs {
    /// The project ID to update.
    id: String,
    /// The new description.
    description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProjectDeleteArgs {
    /// The project ID to delete.
    id: String,
}

#[tool_router]
impl SmbMcpServer {
    #[tool(description = "Get the authenticated smbCloud user's account info. \
                          Requires a prior `smb login`; returns the user as JSON.")]
    async fn me(&self) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let user = me(self.environment, client(), &token)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_result(&user)
    }

    #[tool(description = "List the authenticated user's smbCloud projects as a JSON array.")]
    async fn project_list(&self) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let projects = get_projects(self.environment, client(), token)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_result(&projects)
    }

    #[tool(description = "Show a single smbCloud project by ID, returned as JSON.")]
    async fn project_show(
        &self,
        Parameters(args): Parameters<ProjectShowArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let project = get_project(self.environment, client(), token, args.id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_result(&project)
    }

    #[tool(description = "List deployments for a project by project ID, returned as a JSON array.")]
    async fn deployments(
        &self,
        Parameters(args): Parameters<DeploymentsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let deployments = get_deployments(self.environment, client(), token, args.project_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_result(&deployments)
    }

    #[tool(
        description = "Create a new smbCloud project with a name and optional description. \
                          Returns the created project as JSON."
    )]
    async fn project_create(
        &self,
        Parameters(args): Parameters<ProjectCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let payload = ProjectCreate {
            name: args.name,
            description: args.description,
        };
        let project = create_project(self.environment, client(), token, payload)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_result(&project)
    }

    #[tool(
        description = "Update a project's description by ID, preserving its runner. \
                          Returns the updated project as JSON."
    )]
    async fn project_update(
        &self,
        Parameters(args): Parameters<ProjectUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        // `update_project` requires the runner; fetch the current project so the
        // description-only update doesn't clobber it.
        let current = get_project(self.environment, client(), token.clone(), args.id.clone())
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let project = update_project(
            self.environment,
            client(),
            token,
            args.id,
            &args.description,
            current.runner,
        )
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_result(&project)
    }

    #[tool(
        description = "Delete a project by ID. This is destructive and irreversible — the \
                       project and its deploy configuration are removed."
    )]
    async fn project_delete(
        &self,
        Parameters(args): Parameters<ProjectDeleteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        delete_project(self.environment, client(), token, args.id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            "Project deleted.",
        )]))
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
