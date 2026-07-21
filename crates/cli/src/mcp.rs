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
//!
//! Tools that select a tenant/project (`tenant_use`, `project_use`) persist
//! that choice to the same `~/.smb[-dev]/config.toml` the CLI itself reads
//! (see `crate::session_config`) — an MCP session and a terminal session
//! share state.

use {
    crate::{
        account::lib::is_logged_in,
        client,
        mail::current_project::{resolve_optional_project_id, resolve_required_project_id},
        token::get_smb_token::get_smb_token,
    },
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
    smbcloud_mail::{
        mail_app::{
            create_mail_app, delete_mail_app, get_mail_app, get_mail_apps, update_mail_app,
        },
        mail_inbox::{create_mail_inbox, delete_mail_inbox, send_test_email, update_mail_inbox},
        mail_message::{get_mail_message, get_mail_messages},
    },
    smbcloud_model::{
        app_auth::{AuthAppCreate, AuthAppUpdate},
        mail::{
            MailAppCreate, MailAppUpdate, MailInboxCreate, MailInboxUpdate, MailTestEmailRequest,
        },
        project::ProjectCreate,
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

#[tool_router]
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

    #[tool(description = "List deployments for a project by project ID, returned as a JSON array.")]
    async fn deployments(
        &self,
        Parameters(args): Parameters<DeploymentsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let deployments = get_deployments(self.environment, client(), token, args.project_id)
            .await
            .map_err(to_error_data)?;
        json_result(&deployments)
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

    // ── Mail messages ────────────────────────────────────────────────────

    #[tool(
        description = "List inbound mail messages for an inbox, most recent first, as a JSON array."
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
            args.app_id,
            args.inbox_id,
            Some(args.limit),
        )
        .await
        .map_err(to_error_data)?;
        json_result(&messages)
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
        json_result(&message)
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

    #[tool(description = "Show a single Auth app by ID, returned as JSON.")]
    async fn auth_app_show(
        &self,
        Parameters(args): Parameters<AuthAppShowArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let token = self.access_token()?;
        let auth_app =
            crate::cloud_auth::auth_app::get_auth_app(self.environment, client(), token, args.id)
                .await
                .map_err(to_error_data)?;
        json_result(&auth_app)
    }

    #[tool(
        description = "Create an Auth app under a project. Defaults to the currently \
                       selected project (`project_use`) if project_id is omitted. Returns \
                       the created Auth app as JSON."
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
        json_result(&auth_app)
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
                 `smb login`; tools run non-interactively. `tenant_use` / `project_use` select \
                 the tenant/project context other tools default to, shared with the CLI's own \
                 `smb tenant use` / `smb project use`.",
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
