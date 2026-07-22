# MCP Server

The smbCloud CLI can run as a **Model Context Protocol (MCP) server**, so an AI
assistant or agent — Claude Desktop, Claude Code, Cursor, or any other
MCP-capable client — can manage your smbCloud projects, tenants, Mail apps, and
Auth apps directly, without you leaving the chat to run `smb` commands by hand.
Start it with `smb --mcp`; it speaks standard MCP over stdio and exposes 30
tools covering the account, project, tenant, Mail, and Auth surfaces.

This page is the setup guide and the full tool reference. For how `--mcp`
compares to the CLI's other two interfaces (headless and `--tui`), see
[Interfaces](./interfaces.md). The server is also listed in the official MCP
Registry as `io.github.smbcloudXYZ/smbcloud-cli`, so clients that browse the
registry can install it for you — see [MCP Registry](./mcp-registry.md).

## Prerequisites

1. [Install the CLI](./cli-install.md) (`smb --version` should print something).
2. Log in once, headlessly — the MCP server itself is non-interactive and can't
   run the login flow:

   ```sh
   smb login
   ```

   This writes a token to `~/.smb/token` (`~/.smb-dev/token` in the `dev`
   environment). Every MCP tool call reuses that token; there's nothing
   client-specific to configure for auth.

## Connect your client

Every MCP client ultimately does the same thing: run `smb --mcp` as a
subprocess and speak JSON-RPC over its stdin/stdout. The differences are in
where each client wants that command declared.

### Claude Desktop

Edit your Claude Desktop config —
`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS,
`%APPDATA%\Claude\claude_desktop_config.json` on Windows — and add an entry
under `mcpServers`:

```json
{
  "mcpServers": {
    "smbcloud": {
      "command": "smb",
      "args": ["--mcp"]
    }
  }
}
```

Restart Claude Desktop. The tools listed below become available in any chat.

### Claude Code

```sh
claude mcp add smbcloud -- smb --mcp
```

Or add it directly to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "smbcloud": {
      "command": "smb",
      "args": ["--mcp"]
    }
  }
}
```

### Cursor

Add the same shape to `~/.cursor/mcp.json` (global) or `.cursor/mcp.json`
(project-scoped):

```json
{
  "mcpServers": {
    "smbcloud": {
      "command": "smb",
      "args": ["--mcp"]
    }
  }
}
```

### Any other MCP client

If your client's config isn't listed above, it almost certainly still takes a
`command` + `args` pair for a stdio server — point it at `smb` with `--mcp`
exactly as shown above. That's the entire integration surface; there's no
separate server to run or port to expose.

### Targeting a local API during development

Add the global `-e`/`--environment` flag before `--mcp` to point the server at
a local dev API instead of production:

```json
{
  "mcpServers": {
    "smbcloud-dev": {
      "command": "smb",
      "args": ["-e", "dev", "--mcp"]
    }
  }
}
```

## Session state: tenant and project selection

Several tools take an optional `project_id`, and project creation targets
whichever tenant is currently selected. Two tools manage that selection:

- **`tenant_use`** — selects a tenant (workspace) for the session. While
  selected, `project_create` targets it instead of your personal tenant.
- **`project_use`** — selects a project for the session. Tools with an
  optional `project_id` (Mail apps, Auth apps) default to it when omitted.

Both persist to `~/.smb/config.toml` (`~/.smb-dev/config.toml` in `dev`) — the
same file `smb tenant use` / `smb project use` write from the terminal. Switch
context in a chat with an agent, then run `smb project list` in a terminal a
minute later, and it reflects the same selection. Selecting a tenant that the
current project doesn't belong to clears the project selection, so a stale
cross-tenant `project_id` can never linger silently.

## Tools reference

Every tool returns structured JSON — a single object for `_show`/`_new`/
`_update`/`_use`, an array for `_list`. Fields are the CLI's own JSON models,
not a separate MCP-specific shape.

### Account

| Tool | Arguments | Returns |
| --- | --- | --- |
| `me` | _(none)_ | The authenticated user. |

### Projects

| Tool | Arguments | Returns |
| --- | --- | --- |
| `project_list` | _(none)_ | The user's projects. |
| `project_show` | `id` | A single project by ID. |
| `project_use` | `id` | The selected project. Persists the selection (see above). |
| `project_create` | `name`, `description` (optional) | The created project. Targets the selected tenant, if any. |
| `project_update` | `id`, `description` | The updated project (runner preserved). |
| `project_delete` | `id` | Confirmation. **Destructive and irreversible.** |
| `deployments` | `project_id` | A project's deployments. |

### Tenants

A tenant is a workspace — your personal one, or an organization you belong to.
`tenant_new`/`tenant_delete` only apply to organization tenants; the personal
tenant is bootstrapped on signup and isn't user-managed.

| Tool | Arguments | Returns |
| --- | --- | --- |
| `tenant_list` | _(none)_ | The user's tenants, with role and project count. |
| `tenant_show` | `id` | A single tenant by ID. |
| `tenant_use` | `id` | The selected tenant. Persists the selection (see above). |
| `tenant_new` | `name` | The created organization tenant. |
| `tenant_update` | `id`, `name` | The renamed tenant. |
| `tenant_delete` | `id` | Confirmation. **Destructive and irreversible** — cascades to every project, Mail app, and Auth app the tenant owns. |

### Mail apps

A Mail app owns one domain's mail routing; it belongs to a project.

| Tool | Arguments | Returns |
| --- | --- | --- |
| `mail_list` | `project_id` (optional, defaults to the selected project) | The user's Mail apps. |
| `mail_show` | `id` | A single Mail app by ID. |
| `mail_new` | `name`, `domain`, `project_id` (optional), `aws_region` (optional) | The created Mail app. |
| `mail_update` | `id`, `name`/`domain`/`aws_region` (at least one) | The updated Mail app. |
| `mail_delete` | `id` | Confirmation. **Destructive and irreversible.** |

### Mail inboxes

An inbox route forwards mail arriving at one address under a Mail app's domain.

| Tool | Arguments | Returns |
| --- | --- | --- |
| `mail_inbox_new` | `app_id`, `local_part`, `forward_to_email`, `sender_email` (optional) | The created inbox route. |
| `mail_inbox_update` | `app_id`, `id`, `local_part`/`forward_to_email`/`sender_email` (at least one) | The updated inbox route. |
| `mail_inbox_delete` | `app_id`, `id` | Confirmation. **Destructive and irreversible.** |
| `mail_inbox_test` | `app_id`, `id`, `recipient_email`/`subject`/`body` (all optional) | The test delivery result. |

### Mail messages

Read-only access to an inbox's stored inbound message history.

| Tool | Arguments | Returns |
| --- | --- | --- |
| `mail_message_list` | `app_id`, `inbox_id`, `limit` (optional, default 10) | Recent inbound messages, newest first. |
| `mail_message_show` | `app_id`, `inbox_id`, `id` | A single inbound message. |

### Auth apps

An Auth app is a hosted identity/sign-in service for your own product's end
users; it belongs to a project.

| Tool | Arguments | Returns |
| --- | --- | --- |
| `auth_app_list` | `project_id` (optional, defaults to the selected project) | The user's Auth apps. |
| `auth_app_show` | `id` | A single Auth app by ID. |
| `auth_app_new` | `name`, `project_id` (optional), `support_email` (optional) | The created Auth app. |
| `auth_app_update` | `id`, `name`/`support_email` (at least one) | The updated Auth app. |
| `auth_app_delete` | `id` | Confirmation. **Destructive and irreversible.** |

## Safety: tools run without confirmation

The CLI's own `project delete`, `tenant delete`, and similar commands ask for
confirmation on a terminal — some require typing the resource's name back.
**MCP tools skip that entirely.** MCP is a non-interactive protocol, so a
`*_delete` tool call executes immediately the moment the client sends it. The
calling agent — and the person directing it — is responsible for confirming
intent before invoking a destructive tool, exactly as with any other
irreversible action an assistant can take on your behalf.

If you want a harder guardrail, keep destructive operations out of an agent's
reach at the client level (most MCP clients let you allow/deny individual
tools per server) rather than relying on the server to ask twice.

## Not exposed as tools (yet)

`deploy` and `migrate` are **deliberately not** MCP tools. Both are tightly
coupled to the local working directory in ways a stdio tool call can't satisfy
cleanly:

- **`deploy`** reads `.smb/config.toml` (running interactive `setup_project`
  when it is missing), builds the app locally, then uploads over rsync and
  restarts over SSH. It depends on the caller's current directory, local build
  toolchains, and on-disk SSH keys, and it streams long-running progress — none
  of which map onto a single non-interactive tool call. For automated deploys,
  use the headless CLI path instead (`smb --ci deploy`, see
  [CI / non-interactive deploys](./ci.md)), which is what CI and the deploy
  action already drive.
- **`migrate`** pushes local `.smb/config.toml` deploy fields up to the server,
  so it is meaningless without a project directory to read from.

Exposing either as a tool would mean designing an explicit, directory-free
contract — e.g. accepting the full deploy configuration as arguments and
returning streamed progress as structured events — rather than wrapping the
existing directory-coupled handlers. That's intentionally left for a later
pass; the tools above cover the account, project, tenant, Mail, and Auth
surface that's well-defined over stdio today.

## Talking to it directly

You don't need an MCP client to try this — the server reads newline-delimited
JSON-RPC on stdin and writes responses on stdout. Diagnostics go to stderr and
logging goes to the on-disk log file, so stdout stays a clean protocol channel
end to end. A minimal handshake plus a tool listing:

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | smb --mcp
```

A tool call names the tool and its arguments:

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"project_show","arguments":{"id":"<id>"}}}
```

## FAQ

**Does the MCP server need a different install than the regular CLI?**
No — `smb --mcp` is the same binary you already use for `smb deploy`, `smb
project list`, and so on. There's nothing extra to install.

**Do I need to log in separately for MCP?**
No. Log in once with `smb login` on a terminal; every interface (headless,
`--tui`, `--mcp`) reads the same token from `~/.smb/token`.

**Can I run multiple smbCloud MCP servers for different environments?**
Yes — register two entries with different `args` (e.g. one with `-e dev`, one
without) under different names, as shown above.

**Does selecting a tenant or project in an agent chat affect my terminal?**
Yes, and vice versa — `tenant_use`/`project_use` write to the same
`~/.smb/config.toml` the CLI itself reads, so the two stay in sync.

**Is the MCP server safe to point an autonomous agent at?**
Every write tool — including every `*_delete` tool — executes immediately with
no confirmation prompt, since MCP is non-interactive by design. Treat tool
access the way you'd treat handing someone your terminal with `smb` already
logged in: fine for a trusted agent you're directing, not something to expose
unsupervised.
