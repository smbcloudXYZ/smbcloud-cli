//! MCP (Model Context Protocol) server interface.
//!
//! When `smb` is started with `--mcp`, it runs as an MCP server over stdio
//! instead of executing a one-shot command. The default profile exposes
//! smbCloud operations; `--scope automation` exposes the shared XCRS mobile
//! and TV automation profile.
//!
//! Tools call the same library functions the CLI handlers use, but return
//! structured JSON rather than rendering spinners or a TUI — the stdout stream
//! is the JSON-RPC channel and must stay free of console output. Logging still
//! goes to the on-disk log file (set up by `main`), never to stdout.
//!
//! Tools that select a tenant/project (`tenant_use`, `project_use`) persist
//! that choice to the same `~/.smb[-dev]/config.toml` the CLI itself reads
//! (see `crate::session_config`) — an MCP session and a terminal session
//! share state.

use {
    crate::{
        account::lib::is_logged_in,
        cli::McpScope,
        client,
        mail::{
            current_project::{resolve_optional_project_id, resolve_required_project_id},
            send::{resolve_api_key, send_email, OutboundEmail},
        },
        token::get_smb_token::get_smb_token,
    },
    anyhow::{anyhow, Result},
    rmcp::{
        handler::server::{wrapper::Parameters, ServerHandler},
        model::{
            CallToolResult, ContentBlock, Implementation, ListResourcesResult, MetaObject,
            PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResponse,
            ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerConfig,
        },
        service::RequestContext,
        tool, tool_handler, tool_router, ErrorData, RoleServer, ServiceExt,
    },
    schemars::JsonSchema,
    serde::Deserialize,
    smbcloud_auth::me::me,
    smbcloud_mail::{
        mail_app::{
            create_mail_app, delete_mail_app, get_mail_app, get_mail_apps, update_mail_app,
        },
        mail_inbox::{create_mail_inbox, delete_mail_inbox, send_test_email, update_mail_inbox},
        mail_message::{get_mail_message, get_mail_messages},
    },
    smbcloud_model::{
        app_auth::{AuthApp, AuthAppCreate, AuthAppUpdate},
        mail::{
            MailAppCreate, MailAppUpdate, MailInboxCreate, MailInboxUpdate, MailMessage,
            MailTestEmailRequest,
        },
        project::{Deployment, ProjectCreate},
        tenant::{TenantCreate, TenantUpdate},
    },
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::{
        crud_project_create::create_project,
        crud_project_delete::delete_project,
        crud_project_deployment_read::get_deployments,
        crud_project_read::{get_project, get_projects},
        crud_project_update::update_project,
    },
};

/// The smbCloud MCP server. Holds the environment selected on the command line
/// so every tool talks to the same API host and on-disk state dir.
pub struct SmbMcpServer {
    environment: Environment,
}

#[path = "mcp_xcrs.rs"]
mod mcp_xcrs;

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
        get_smb_token(self.environment).map_err(to_error_data)
    }
}

/// Serialize any JSON-able value into a single-content successful tool result.
fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::json(value)?]))
}

fn text_result(message: impl Into<String>) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::text(
        message.into(),
    )]))
}

fn to_error_data(error: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}

fn invalid_request(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_request(message.into(), None)
}

const DEPLOYMENTS_VIEW_URI: &str = "ui://smbcloud/deployments.html";
const INBOX_VIEW_URI: &str = "ui://smbcloud/inbox.html";
const AUTH_APP_VIEW_URI: &str = "ui://smbcloud/auth-app.html";
const MCP_APP_MIME_TYPE: &str = "text/html;profile=mcp-app";

/// Every MCP Apps view the server serves: (uri, name, description, html).
/// Adding a view is one row here plus the tool that points at it.
const VIEWS: [(&str, &str, &str, &str); 3] = [
    (
        DEPLOYMENTS_VIEW_URI,
        "deployments",
        "Interactive deployments dashboard for a project.",
        include_str!("mcp_apps/deployments.html"),
    ),
    (
        INBOX_VIEW_URI,
        "inbox",
        "Interactive inbox for a Mail inbox route.",
        include_str!("mcp_apps/inbox.html"),
    ),
    (
        AUTH_APP_VIEW_URI,
        "auth-app",
        "Setup card for an Auth app: client ID, secret, and provider checklist.",
        include_str!("mcp_apps/auth_app.html"),
    ),
];

/// List views show at most this many of the newest rows, which also keeps the
/// text fallback under host size limits.
const VIEW_ROW_CAP: usize = 50;

/// `_meta` linking a tool to its MCP Apps view (SEP-1865).
///
/// Sent unconditionally rather than gated on the client's
/// `io.modelcontextprotocol/ui` extension: hosts without MCP Apps support
/// ignore `_meta`, and gating would need per-session state in the tool router
/// for no user-visible gain.
///
/// The URI is emitted under both `_meta.ui.resourceUri` and the legacy flat
/// `_meta["ui/resourceUri"]`, because older hosts read only the flat key.
fn ui_tool_meta(uri: &str) -> MetaObject {
    let mut meta = MetaObject::new();
    meta.insert("ui".to_string(), serde_json::json!({ "resourceUri": uri }));
    meta.insert("ui/resourceUri".to_string(), serde_json::json!(uri));
    meta
}

fn deployments_tool_meta() -> MetaObject {
    ui_tool_meta(DEPLOYMENTS_VIEW_URI)
}

fn inbox_tool_meta() -> MetaObject {
    ui_tool_meta(INBOX_VIEW_URI)
}

fn auth_app_tool_meta() -> MetaObject {
    ui_tool_meta(AUTH_APP_VIEW_URI)
}

/// The plain JSON content stays so hosts without MCP Apps support (and the
/// spec's requirement for a meaningful `content`) keep working; the view reads
/// `structured_content`, which must be an object rather than a bare array.
fn deployments_result(
    project_id: i32,
    deployments: &[Deployment],
) -> Result<CallToolResult, ErrorData> {
    let mut newest: Vec<&Deployment> = deployments.iter().collect();
    newest.sort_by_key(|deployment| std::cmp::Reverse(deployment.created_at));
    newest.truncate(VIEW_ROW_CAP);
    let mut result = CallToolResult::success(vec![ContentBlock::json(&newest)?]);
    result.structured_content = Some(serde_json::json!({
        "project_id": project_id,
        "deployments": newest,
    }));
    Ok(result)
}

fn mail_messages_result(
    app_id: &str,
    inbox_id: &str,
    messages: &[MailMessage],
) -> Result<CallToolResult, ErrorData> {
    let mut newest: Vec<&MailMessage> = messages.iter().collect();
    newest.sort_by_key(|message| std::cmp::Reverse(message.received_at));
    newest.truncate(VIEW_ROW_CAP);
    let mut result = CallToolResult::success(vec![ContentBlock::json(&newest)?]);
    result.structured_content = Some(serde_json::json!({
        "app_id": app_id,
        "inbox_id": inbox_id,
        "messages": newest,
    }));
    Ok(result)
}

fn auth_app_result(auth_app: &AuthApp) -> Result<CallToolResult, ErrorData> {
    let mut result = CallToolResult::success(vec![ContentBlock::json(auth_app)?]);
    result.structured_content = Some(serde_json::json!({ "auth_app": auth_app }));
    Ok(result)
}

fn mail_message_result(message: &MailMessage) -> Result<CallToolResult, ErrorData> {
    let mut result = CallToolResult::success(vec![ContentBlock::json(message)?]);
    result.structured_content = Some(serde_json::json!({ "message": message }));
    Ok(result)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProjectShowArgs {
    /// The project ID to show.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProjectUseArgs {
    /// The project ID to select for this session.
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

#[derive(Debug, Deserialize, JsonSchema)]
struct TenantShowArgs {
    /// The tenant ID to show.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TenantNewArgs {
    /// Name for the new organization tenant.
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TenantUpdateArgs {
    /// The tenant ID to rename.
    id: String,
    /// The new tenant name.
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TenantDeleteArgs {
    /// The tenant ID to delete. Must be an organization tenant — the
    /// personal tenant can't be deleted.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TenantUseArgs {
    /// The tenant ID to select for this session. Project creation and other
    /// tenant-scoped operations default to this tenant instead of the
    /// personal one.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailAppListArgs {
    /// Restrict to mail apps under this project ID. Defaults to the
    /// currently selected project (`project_use`) if omitted; lists across
    /// all projects if none is selected.
    #[serde(default)]
    project_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailAppShowArgs {
    /// The mail app ID to show.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailAppNewArgs {
    /// Name for the new mail app.
    name: String,
    /// The domain the mail app will send/receive on.
    domain: String,
    /// Project ID to create the mail app under. Defaults to the currently
    /// selected project (`project_use`) if omitted.
    #[serde(default)]
    project_id: Option<String>,
    /// AWS region for SES provisioning. Defaults to the API's default region.
    #[serde(default)]
    aws_region: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailAppUpdateArgs {
    /// The mail app ID to update.
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    aws_region: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailAppDeleteArgs {
    /// The mail app ID to delete.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailInboxNewArgs {
    /// The mail app ID to create the inbox route under.
    app_id: String,
    /// The local part of the inbox address (before the `@`).
    local_part: String,
    /// The email address inbound mail is forwarded to.
    forward_to_email: String,
    /// Sender email used for outbound forwards. Defaults to the mail app's
    /// configured sender.
    #[serde(default)]
    sender_email: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailInboxUpdateArgs {
    /// The mail app ID the inbox route belongs to.
    app_id: String,
    /// The inbox route ID to update.
    id: String,
    #[serde(default)]
    local_part: Option<String>,
    #[serde(default)]
    forward_to_email: Option<String>,
    #[serde(default)]
    sender_email: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailInboxDeleteArgs {
    /// The mail app ID the inbox route belongs to.
    app_id: String,
    /// The inbox route ID to delete.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailInboxTestArgs {
    /// The mail app ID the inbox route belongs to.
    app_id: String,
    /// The inbox route ID to send a test email through.
    id: String,
    /// Recipient for the test email. Defaults to the inbox's forward target.
    #[serde(default)]
    recipient_email: Option<String>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailSendArgs {
    /// Sender address. Must sit on the verified domain of the Mail app the
    /// configured API key belongs to.
    from: String,
    /// One or more recipient addresses.
    to: Vec<String>,
    #[serde(default)]
    subject: Option<String>,
    /// HTML body. At least one of `html` or `text` is required.
    #[serde(default)]
    html: Option<String>,
    /// Plain-text body. At least one of `html` or `text` is required.
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    cc: Vec<String>,
    #[serde(default)]
    bcc: Vec<String>,
    #[serde(default)]
    reply_to: Vec<String>,
    /// Reuse a key to make a retry safe: the original message comes back
    /// instead of a second send.
    #[serde(default)]
    idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailMessageListArgs {
    /// The mail app ID the inbox belongs to.
    app_id: String,
    /// The inbox route ID whose messages to list.
    inbox_id: String,
    /// Max number of messages to return, most recent first.
    #[serde(default = "default_message_limit")]
    limit: u32,
}

fn default_message_limit() -> u32 {
    10
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MailMessageShowArgs {
    /// The mail app ID the inbox belongs to.
    app_id: String,
    /// The inbox route ID the message belongs to.
    inbox_id: String,
    /// The message ID to show.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AuthAppListArgs {
    /// Restrict to Auth apps under this project ID.
    #[serde(default)]
    project_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AuthAppShowArgs {
    /// The Auth app ID to show.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AuthAppNewArgs {
    /// Name for the new Auth app.
    name: String,
    /// Project ID to create the Auth app under. Defaults to the currently
    /// selected project (`project_use`) if omitted.
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    support_email: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AuthAppUpdateArgs {
    /// The Auth app ID to update.
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    support_email: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AuthAppDeleteArgs {
    /// The Auth app ID to delete.
    id: String,
}

#[tool_router(router = cloud_tool_router, vis = "pub(crate)")]
impl SmbMcpServer {
    #[tool(description = "Get the authenticated smbCloud user's account info. \
                          Requires a prior `smb login`; returns the user as JSON.")]
    async fn me(&self) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let user = me(self.environment, client(), &token)
            .await
            .map_err(to_error_data)?;
        json_result(&user)
    }

    // ── Projects ─────────────────────────────────────────────────────────

    #[tool(description = "List the authenticated user's smbCloud projects as a JSON array.")]
    async fn project_list(&self) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let projects = get_projects(self.environment, client(), token)
            .await
            .map_err(to_error_data)?;
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
            .map_err(to_error_data)?;
        json_result(&project)
    }

    #[tool(
        description = "Select a project for this session, persisted to the same config \
                       CLI sessions read. Mail/Auth tools that take an optional project_id \
                       default to this project when omitted. Returns the selected project as JSON."
    )]
    async fn project_use(
        &self,
        Parameters(args): Parameters<ProjectUseArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let project = get_project(self.environment, client(), token, args.id)
            .await
            .map_err(to_error_data)?;
        // MCP tools are non-interactive, so unlike `smb project use` we don't
        // try to resolve/select a frontend app here.
        crate::session_config::set_current_project(self.environment, project.clone(), None)
            .map_err(to_error_data)?;
        json_result(&project)
    }

    #[tool(
        description = "List deployments for a project by project ID, returned as a JSON array. \
                       Hosts that support MCP Apps render an interactive dashboard.",
        meta = deployments_tool_meta()
    )]
    async fn deployments(
        &self,
        Parameters(args): Parameters<DeploymentsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let deployments = get_deployments(self.environment, client(), token, args.project_id)
            .await
            .map_err(to_error_data)?;
        deployments_result(args.project_id, &deployments)
    }

    #[tool(
        description = "Create a new smbCloud project with a name and optional description. \
                       Created under the currently selected tenant (`tenant_use`), or the \
                       user's personal tenant if none is selected. Returns the created \
                       project as JSON."
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
        let tenant_id = crate::session_config::current_tenant_id(self.environment).unwrap_or(None);
        let project = create_project(self.environment, client(), token, payload, tenant_id)
            .await
            .map_err(to_error_data)?;
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
            .map_err(to_error_data)?;
        let project = update_project(
            self.environment,
            client(),
            token,
            args.id,
            &args.description,
            current.runner,
        )
        .await
        .map_err(to_error_data)?;
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
            .map_err(to_error_data)?;
        text_result("Project deleted.")
    }

    // ── Tenants ──────────────────────────────────────────────────────────

    #[tool(
        description = "List the authenticated user's smbCloud tenants (workspaces) as a JSON array."
    )]
    async fn tenant_list(&self) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let tenants = crate::tenant::tenant_client::get_tenants(self.environment, client(), token)
            .await
            .map_err(to_error_data)?;
        json_result(&tenants)
    }

    #[tool(description = "Show a single smbCloud tenant by ID, returned as JSON.")]
    async fn tenant_show(
        &self,
        Parameters(args): Parameters<TenantShowArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let tenant =
            crate::tenant::tenant_client::get_tenant(self.environment, client(), token, args.id)
                .await
                .map_err(to_error_data)?;
        json_result(&tenant)
    }

    #[tool(
        description = "Create a new organization tenant (workspace). Personal tenants are \
                       bootstrapped on signup and can't be created here. Returns the created \
                       tenant as JSON."
    )]
    async fn tenant_new(
        &self,
        Parameters(args): Parameters<TenantNewArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let tenant = crate::tenant::tenant_client::create_tenant(
            self.environment,
            client(),
            token,
            TenantCreate { name: args.name },
        )
        .await
        .map_err(to_error_data)?;
        json_result(&tenant)
    }

    #[tool(description = "Rename a tenant by ID. Returns the updated tenant as JSON.")]
    async fn tenant_update(
        &self,
        Parameters(args): Parameters<TenantUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let tenant = crate::tenant::tenant_client::update_tenant(
            self.environment,
            client(),
            token,
            args.id,
            TenantUpdate {
                name: Some(args.name),
            },
        )
        .await
        .map_err(to_error_data)?;
        json_result(&tenant)
    }

    #[tool(
        description = "Delete an organization tenant by ID. This is destructive and \
                       irreversible — it cascades to every project, Auth app, and Mail \
                       app the tenant owns. The personal tenant can't be deleted."
    )]
    async fn tenant_delete(
        &self,
        Parameters(args): Parameters<TenantDeleteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        crate::tenant::tenant_client::delete_tenant(self.environment, client(), token, args.id)
            .await
            .map_err(to_error_data)?;
        text_result("Tenant deleted.")
    }

    #[tool(
        description = "Select a tenant for this session, persisted to the same config CLI \
                       sessions read. `project_create` targets this tenant instead of the \
                       personal one while it's selected. Clears the selected project if it \
                       belongs to a different tenant. Returns the selected tenant as JSON."
    )]
    async fn tenant_use(
        &self,
        Parameters(args): Parameters<TenantUseArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let tenant =
            crate::tenant::tenant_client::get_tenant(self.environment, client(), token, args.id)
                .await
                .map_err(to_error_data)?;
        crate::session_config::set_current_tenant(self.environment, tenant.clone())
            .map_err(to_error_data)?;
        json_result(&tenant)
    }

    // ── Mail apps ────────────────────────────────────────────────────────

    #[tool(description = "List the authenticated user's smbCloud Mail apps as a JSON array.")]
    async fn mail_list(
        &self,
        Parameters(args): Parameters<MailAppListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let project_id = resolve_optional_project_id(self.environment, args.project_id)
            .map_err(to_error_data)?;
        let mail_apps = get_mail_apps(self.environment, client(), token, project_id)
            .await
            .map_err(to_error_data)?;
        json_result(&mail_apps)
    }

    #[tool(description = "Show a single Mail app by ID, returned as JSON.")]
    async fn mail_show(
        &self,
        Parameters(args): Parameters<MailAppShowArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let mail_app = get_mail_app(self.environment, client(), token, args.id)
            .await
            .map_err(to_error_data)?;
        json_result(&mail_app)
    }

    #[tool(
        description = "Create a Mail app for a domain under a project. Defaults to the \
                       currently selected project (`project_use`) if project_id is omitted. \
                       Returns the created Mail app as JSON."
    )]
    async fn mail_new(
        &self,
        Parameters(args): Parameters<MailAppNewArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let project_id = resolve_required_project_id(self.environment, args.project_id)
            .map_err(to_error_data)?;
        let mail_app = create_mail_app(
            self.environment,
            client(),
            token,
            MailAppCreate {
                name: args.name,
                project_id,
                domain: args.domain,
                aws_region: args.aws_region,
            },
        )
        .await
        .map_err(to_error_data)?;
        json_result(&mail_app)
    }

    #[tool(
        description = "Update a Mail app's name, domain, and/or AWS region by ID. Returns the updated Mail app as JSON."
    )]
    async fn mail_update(
        &self,
        Parameters(args): Parameters<MailAppUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let update = MailAppUpdate {
            name: args.name,
            domain: args.domain,
            aws_region: args.aws_region,
        };
        if update.is_empty() {
            return Err(invalid_request(
                "Specify at least one of name, domain, or aws_region.",
            ));
        }
        let mail_app = update_mail_app(self.environment, client(), token, args.id, update)
            .await
            .map_err(to_error_data)?;
        json_result(&mail_app)
    }

    #[tool(
        description = "Delete a Mail app by ID. This is destructive and irreversible — its \
                       inbox routes and SES routing are removed."
    )]
    async fn mail_delete(
        &self,
        Parameters(args): Parameters<MailAppDeleteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        delete_mail_app(self.environment, client(), token, args.id)
            .await
            .map_err(to_error_data)?;
        text_result("Mail app deleted.")
    }

    // ── Mail inboxes ─────────────────────────────────────────────────────

    #[tool(
        description = "Create a mail inbox route under a Mail app. Returns the created inbox route as JSON."
    )]
    async fn mail_inbox_new(
        &self,
        Parameters(args): Parameters<MailInboxNewArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let inbox = create_mail_inbox(
            self.environment,
            client(),
            token,
            args.app_id,
            MailInboxCreate {
                local_part: args.local_part,
                forward_to_email: args.forward_to_email,
                sender_email: args.sender_email,
            },
        )
        .await
        .map_err(to_error_data)?;
        json_result(&inbox)
    }

    #[tool(
        description = "Update a mail inbox route's local part, forward target, and/or sender email. Returns the updated inbox route as JSON."
    )]
    async fn mail_inbox_update(
        &self,
        Parameters(args): Parameters<MailInboxUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let update = MailInboxUpdate {
            local_part: args.local_part,
            forward_to_email: args.forward_to_email,
            sender_email: args.sender_email,
        };
        if update.is_empty() {
            return Err(invalid_request(
                "Specify at least one of local_part, forward_to_email, or sender_email.",
            ));
        }
        let inbox = update_mail_inbox(
            self.environment,
            client(),
            token,
            args.app_id,
            args.id,
            update,
        )
        .await
        .map_err(to_error_data)?;
        json_result(&inbox)
    }

    #[tool(description = "Delete a mail inbox route. This is destructive and irreversible.")]
    async fn mail_inbox_delete(
        &self,
        Parameters(args): Parameters<MailInboxDeleteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        delete_mail_inbox(self.environment, client(), token, args.app_id, args.id)
            .await
            .map_err(to_error_data)?;
        text_result("Mail inbox route deleted.")
    }

    #[tool(
        description = "Send a test email through a mail inbox route to verify forwarding. Returns the delivery result as JSON."
    )]
    async fn mail_inbox_test(
        &self,
        Parameters(args): Parameters<MailInboxTestArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let delivery = send_test_email(
            self.environment,
            client(),
            token,
            args.app_id,
            args.id,
            MailTestEmailRequest {
                recipient_email: args.recipient_email,
                subject: args.subject,
                body: args.body,
            },
        )
        .await
        .map_err(to_error_data)?;
        json_result(&delivery)
    }

    #[tool(
        description = "Send a transactional email from a verified smbCloud Mail domain. \
                       Returns the created message with its delivery status as JSON. \
                       This tool authenticates with a Mail app API key from \
                       SMB_MAIL_API_KEY, not the `smb login` session the other tools use."
    )]
    async fn mail_send(
        &self,
        Parameters(args): Parameters<MailSendArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let api_key = resolve_api_key(None).map_err(|error| invalid_request(error.to_string()))?;

        let sent = send_email(
            self.environment,
            &api_key,
            OutboundEmail {
                from: args.from,
                to: args.to,
                subject: args.subject,
                html: args.html,
                text: args.text,
                cc: args.cc,
                bcc: args.bcc,
                reply_to: args.reply_to,
                idempotency_key: args.idempotency_key,
            },
        )
        .await
        .map_err(to_error_data)?;

        json_result(&sent)
    }

    // ── Mail messages ────────────────────────────────────────────────────

    #[tool(
        description = "List inbound mail messages for an inbox, most recent first, as a JSON array \
                       (at most the newest 50). Hosts that support MCP Apps render an \
                       interactive inbox.",
        meta = inbox_tool_meta()
    )]
    async fn mail_message_list(
        &self,
        Parameters(args): Parameters<MailMessageListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let messages = get_mail_messages(
            self.environment,
            client(),
            token,
            args.app_id.clone(),
            args.inbox_id.clone(),
            Some(args.limit.min(VIEW_ROW_CAP as u32)),
        )
        .await
        .map_err(to_error_data)?;
        mail_messages_result(&args.app_id, &args.inbox_id, &messages)
    }

    #[tool(description = "Show a single inbound mail message, returned as JSON.")]
    async fn mail_message_show(
        &self,
        Parameters(args): Parameters<MailMessageShowArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let message = get_mail_message(
            self.environment,
            client(),
            token,
            args.app_id,
            args.inbox_id,
            args.id,
        )
        .await
        .map_err(to_error_data)?;
        mail_message_result(&message)
    }

    // ── Auth apps ────────────────────────────────────────────────────────

    #[tool(description = "List the authenticated user's smbCloud Auth apps as a JSON array.")]
    async fn auth_app_list(
        &self,
        Parameters(args): Parameters<AuthAppListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let project_id = resolve_optional_project_id(self.environment, args.project_id)
            .map_err(to_error_data)?;
        let auth_apps = crate::cloud_auth::auth_app::get_auth_apps(
            self.environment,
            client(),
            token,
            project_id,
        )
        .await
        .map_err(to_error_data)?;
        json_result(&auth_apps)
    }

    #[tool(
        description = "Show a single Auth app by ID, returned as JSON. Hosts that support \
                       MCP Apps render a setup card.",
        meta = auth_app_tool_meta()
    )]
    async fn auth_app_show(
        &self,
        Parameters(args): Parameters<AuthAppShowArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let auth_app =
            crate::cloud_auth::auth_app::get_auth_app(self.environment, client(), token, args.id)
                .await
                .map_err(to_error_data)?;
        auth_app_result(&auth_app)
    }

    #[tool(
        description = "Create an Auth app under a project. Defaults to the currently \
                       selected project (`project_use`) if project_id is omitted. Returns \
                       the created Auth app as JSON. Hosts that support MCP Apps render a \
                       setup card.",
        meta = auth_app_tool_meta()
    )]
    async fn auth_app_new(
        &self,
        Parameters(args): Parameters<AuthAppNewArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let project_id = resolve_required_project_id(self.environment, args.project_id)
            .map_err(to_error_data)?;
        let auth_app = crate::cloud_auth::auth_app::create_auth_app(
            self.environment,
            client(),
            token,
            AuthAppCreate {
                name: args.name,
                project_id,
                support_email: args.support_email,
            },
        )
        .await
        .map_err(to_error_data)?;
        auth_app_result(&auth_app)
    }

    #[tool(
        description = "Update an Auth app's name and/or support email by ID. Returns the updated Auth app as JSON."
    )]
    async fn auth_app_update(
        &self,
        Parameters(args): Parameters<AuthAppUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let update = AuthAppUpdate {
            name: args.name,
            support_email: args.support_email,
        };
        if update.is_empty() {
            return Err(invalid_request(
                "Specify at least one of name or support_email.",
            ));
        }
        let auth_app = crate::cloud_auth::auth_app::update_auth_app(
            self.environment,
            client(),
            token,
            args.id,
            update,
        )
        .await
        .map_err(to_error_data)?;
        json_result(&auth_app)
    }

    #[tool(
        description = "Delete an Auth app by ID. This is destructive and irreversible — its \
                       OAuth clients and configuration are removed."
    )]
    async fn auth_app_delete(
        &self,
        Parameters(args): Parameters<AuthAppDeleteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        crate::cloud_auth::auth_app::delete_auth_app(self.environment, client(), token, args.id)
            .await
            .map_err(to_error_data)?;
        text_result("Auth app deleted.")
    }
}

#[tool_handler(router = Self::cloud_tool_router())]
impl ServerHandler for SmbMcpServer {
    fn get_info(&self) -> ServerConfig {
        // `Implementation` is `#[non_exhaustive]`, so start from the build-env
        // default and override the identity fields to report `smb`, not `rmcp`.
        let mut server_info = Implementation::from_build_env();
        server_info.name = "smb".to_string();
        server_info.version = env!("CARGO_PKG_VERSION").to_string();

        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(server_info)
        .with_instructions(
            "smbCloud CLI exposed as MCP tools. Authentication uses the token stored by \
                 `smb login`; tools run non-interactively. `tenant_use` / `project_use` select \
                 the tenant/project context other tools default to, shared with the CLI's own \
                 `smb tenant use` / `smb project use`. Use `smb --mcp --scope automation` \
                 instead for mobile and TV app automation tools. `deployments` renders an \
                 interactive dashboard on MCP Apps hosts.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let views = VIEWS
            .iter()
            .map(|(uri, name, description, _)| {
                Resource::new(*uri, *name)
                    .with_description(*description)
                    .with_mime_type(MCP_APP_MIME_TYPE)
            })
            .collect();
        Ok(ListResourcesResult::with_all_items(views))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let Some((_, _, _, html)) = VIEWS.iter().find(|(uri, ..)| *uri == request.uri) else {
            return Err(ErrorData::resource_not_found(
                format!("Unknown resource: {}", request.uri),
                None,
            ));
        };
        // No `csp` on purpose: the view loads nothing external, so the host's
        // restrictive default policy is the right one.
        let mut meta = MetaObject::new();
        meta.insert(
            "ui".to_string(),
            serde_json::json!({ "prefersBorder": true }),
        );
        let contents = ResourceContents::text(*html, request.uri)
            .with_mime_type(MCP_APP_MIME_TYPE)
            .with_meta(meta);
        Ok(ReadResourceResult::new(vec![contents]).into())
    }
}

/// Run the MCP server over stdio until the client disconnects.
pub async fn serve(environment: Environment, scope: McpScope) -> Result<()> {
    if scope == McpScope::Automation {
        return mcp_xcrs::serve().await;
    }

    let running = SmbMcpServer::new(environment)
        .serve(xcrs::mcp::compatible_stdio().await?)
        .await
        .map_err(|e| anyhow!("Failed to start MCP server: {e}"))?;
    running
        .waiting()
        .await
        .map_err(|e| anyhow!("MCP server stopped unexpectedly: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_router_exposes_known_cloud_tools() {
        let tools = SmbMcpServer::cloud_tool_router().list_all();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();

        for cloud_only in [
            "me",
            "project_list",
            "tenant_list",
            "mail_list",
            "auth_app_list",
        ] {
            assert!(
                names.contains(&cloud_only),
                "cloud router should expose {cloud_only:?}"
            );
        }
    }

    #[test]
    fn view_tools_declare_ui_resource_under_both_keys() {
        let tools = SmbMcpServer::cloud_tool_router().list_all();
        for (tool_name, uri) in [
            ("deployments", DEPLOYMENTS_VIEW_URI),
            ("mail_message_list", INBOX_VIEW_URI),
            ("auth_app_show", AUTH_APP_VIEW_URI),
            ("auth_app_new", AUTH_APP_VIEW_URI),
        ] {
            let tool = tools
                .iter()
                .find(|tool| tool.name == tool_name)
                .unwrap_or_else(|| panic!("{tool_name} tool"));
            let meta = tool.meta.as_ref().expect("tool _meta");
            assert_eq!(meta["ui"]["resourceUri"], uri, "{tool_name}");
            assert_eq!(meta["ui/resourceUri"], uri, "{tool_name}");
        }
    }

    #[test]
    fn views_are_self_contained() {
        for (uri, _, _, html) in VIEWS {
            assert!(!html.is_empty(), "{uri}");
            assert!(html.contains("ui/initialize"), "{uri}");
            assert!(!html.contains("http://"), "{uri}");
            assert!(!html.contains("https://"), "{uri}");
        }
    }

    #[test]
    fn deployments_result_carries_content_and_structured_content() {
        let deployments: Vec<Deployment> = serde_json::from_value(serde_json::json!([{
            "id": 1,
            "project_id": 7,
            "frontend_app_id": null,
            "frontend_app_name": "web",
            "commit_hash": "abcdef1234567",
            "status": 2,
            "created_at": "2026-01-01T00:00:00.000+00:00",
            "updated_at": "2026-01-01T00:01:00.000+00:00",
        }]))
        .expect("fixture deployment");

        let result = deployments_result(7, &deployments).expect("result");
        assert!(!result.content.is_empty());
        let structured = result.structured_content.expect("structured content");
        assert_eq!(structured["project_id"], 7);
        assert_eq!(
            structured["deployments"]
                .as_array()
                .map(|items| items.len()),
            Some(1)
        );
    }

    fn fixture_message(index: usize) -> MailMessage {
        serde_json::from_value(serde_json::json!({
            "id": format!("message-{index}"),
            "mail_inbox_id": "inbox",
            "provider_message_id": "provider",
            "original_recipient_email": "to@example.com",
            "from_email": "from@example.com",
            "from_name": null,
            "subject": "Hello",
            "text_preview": "Preview",
            "size_bytes": null,
            "forwarded_at": null,
            "forward_error": null,
            "received_at": format!("2026-01-01T00:{:02}:00Z", index % 60),
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "status": 0,
            "text_body": "Body",
            "html_body": null,
            "s3_bucket": null,
            "s3_key": null,
        }))
        .expect("fixture message")
    }

    #[test]
    fn mail_messages_result_carries_content_and_structured_content() {
        let messages: Vec<MailMessage> = (0..60).map(fixture_message).collect();

        let result = mail_messages_result("app", "inbox", &messages).expect("result");
        assert!(!result.content.is_empty());
        let structured = result.structured_content.expect("structured content");
        assert_eq!(structured["app_id"], "app");
        assert_eq!(structured["inbox_id"], "inbox");
        assert_eq!(
            structured["messages"].as_array().map(|items| items.len()),
            Some(VIEW_ROW_CAP)
        );
    }

    #[test]
    fn auth_app_result_carries_content_and_structured_content() {
        let auth_app: AuthApp = serde_json::from_value(serde_json::json!({
            "id": "client-id",
            "secret": "client-secret",
            "name": "My app",
            "project_id": 7,
            "support_email": null,
            "apple_oauth_enabled": true,
            "apple_oauth_client_id": null,
            "apple_oauth_configured": false,
            "apple_oauth_secret_present": false,
            "created_at": "2026-01-01T00:00:00.000+00:00",
            "updated_at": "2026-01-01T00:01:00.000+00:00",
        }))
        .expect("fixture auth app");

        let result = auth_app_result(&auth_app).expect("result");
        assert!(!result.content.is_empty());
        let structured = result.structured_content.expect("structured content");
        assert_eq!(structured["auth_app"]["id"], "client-id");
    }

    #[test]
    fn mail_message_result_carries_content_and_structured_content() {
        let result = mail_message_result(&fixture_message(1)).expect("result");
        assert!(!result.content.is_empty());
        let structured = result.structured_content.expect("structured content");
        assert_eq!(structured["message"]["id"], "message-1");
    }

    #[test]
    fn cloud_router_excludes_automation_tools() {
        let tools = SmbMcpServer::cloud_tool_router().list_all();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();

        const AUTOMATION_TOOL_NAMES: [&str; 18] = [
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

        for automation_only in AUTOMATION_TOOL_NAMES {
            assert!(
                !names.contains(&automation_only),
                "cloud router should not expose automation tool {automation_only:?}"
            );
        }
    }
}
