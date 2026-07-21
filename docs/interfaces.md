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
forces non-interactive behavior (see
[CI / non-interactive deploys](./ci.md)) and applies to the
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

`smb --mcp` starts an MCP (Model Context Protocol) server that speaks
JSON-RPC over stdio. Instead of running a single command and exiting, it stays
up and exposes smbCloud operations — 30 tools spanning accounts, projects,
tenants, Mail, and Auth — as MCP **tools** that an MCP-capable client
(Claude Desktop, Claude Code, Cursor, or any other assistant/agent) can call.
The subcommand is ignored in this mode.

Authentication uses the token stored by `smb login`, so log in once before
starting the server. Tools run non-interactively and return structured JSON —
including the `*_delete` tools, which apply immediately with no confirmation
prompt.

For the client setup guide and the full tool reference, see
[MCP Server](./mcp.md).
