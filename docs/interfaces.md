# Interfaces: headless, TUI, and MCP

`smb` presents through one of three interfaces, chosen at runtime. The default
is **headless** — plain, line-based text that works the same in a terminal, a
pipe, or a script. Two global flags switch interface:

| Flag | Interface | What it does |
|---|---|---|
| _(none)_ | **Headless** (default) | Plain-text output. Interactive prompts still appear on a TTY unless `--ci` is set. No full-screen takeover. |
| `--tui` | **TUI** | Full-screen interactive views (`ratatui`) for the read commands. |
| `--mcp` | **MCP** | Runs as a Model Context Protocol server over stdio instead of a one-shot command. Implies non-interactive. |

`--tui` and `--mcp` are mutually exclusive. The `--ci` flag is orthogonal: it
forces non-interactive behavior (see [ci.md](./ci.md)) and applies to the
headless interface; `--mcp` is always non-interactive regardless of `--ci`.

## Headless (default)

The default interface prints plain text and never seizes the terminal, so it is
safe to pipe or redirect:

```sh
smb me                            # plain key/value account block
smb project list                  # plain table
smb project show --id <id>        # plain detail block
smb project deployment            # plain table (or detail with --id <id>)
```

On a real terminal, commands that need input still prompt (project setup, a
monorepo target picker, delete confirmations). Add `--ci` — or run under CI — to
turn those prompts off and fail fast instead of blocking.

## TUI (`--tui`)

`--tui` renders the read commands as full-screen interactive views. Press
`q` / `Esc` to leave a view and return to the shell.

```sh
smb --tui me
smb --tui project list
smb --tui project show --id <id>
smb --tui project deployment
```

Destructive confirmations (e.g. `project delete`) show a full-screen danger
dialog under `--tui`; in the headless interface they ask inline instead.

## MCP (`--mcp`)

`smb --mcp` starts an MCP server that speaks JSON-RPC over stdio. Instead of
running a single command and exiting, it stays up and exposes smbCloud
operations as MCP **tools** that an MCP-capable client (assistant, IDE, agent)
can call. The subcommand is ignored in this mode.

Authentication uses the token stored by `smb login`, so log in once before
starting the server. Tools run non-interactively and return structured JSON.

### Available tools

| Tool | Arguments | Returns |
|---|---|---|
| `me` | _(none)_ | The authenticated user. |
| `project_list` | _(none)_ | The user's projects. |
| `project_show` | `id` | A single project by ID. |
| `deployments` | `project_id` | A project's deployments. |
| `project_create` | `name`, `description` (optional) | The created project. |
| `project_update` | `id`, `description` | The updated project (runner preserved). |
| `project_delete` | `id` | Confirmation. **Destructive and irreversible.** |

Tools run non-interactively, so the write tools apply immediately without a
confirmation prompt — the calling client is responsible for confirming intent
before invoking `project_delete`.

### Wiring it into an MCP client

Most MCP clients take a command plus arguments. Point the client at the `smb`
binary with `--mcp`:

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

To target a local API during development, add the environment flag —
`"args": ["-e", "dev", "--mcp"]`.

### Talking to it directly

The server reads newline-delimited JSON-RPC on stdin and writes responses on
stdout. Diagnostics go to stderr and logging goes to the on-disk log file, so
stdout stays a clean protocol channel. A minimal handshake:

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
