# Stability and 1.0

This page says what the 1.0 release promises, why we think `smb` is ready to make that promise, and what is still open.

## What 1.0 freezes

From 1.0.0, these only change in a major release:

- Command names, subcommands and flags.
- Exit codes. `smb` exits 0 on success and 1 on any failure.
- The `.smb/config.toml` schema: existing keys keep their meaning, and a config written for 1.0 keeps working across 1.x.
- MCP tool names and argument shapes, for both the cloud profile (`smb --mcp`) and the automation profile (`smb --mcp --scope automation`).
- `--ci` behavior: with `--ci`, `SMB_CI=1` or a provider-set `CI`, confirmations take their default and anything that needs input fails fast instead of waiting on a terminal.

New commands, flags, tools, optional arguments and config keys can appear in a minor release. Human-readable output text is not part of the contract. Scripts should rely on exit codes and the MCP tool results.

## Why the CLI is stable on the platform side

smbCloud is headless-first. Every capability has to work without a browser, and it reaches users in this order: REST API, then `smb`, then MCP. That makes `smb` and its MCP server the contract that scripts and agents build against, so the platform has a reason to keep them steady.

- **One change, three surfaces.** A console capability is required to land in the API, the CLI and the MCP server in the same change. The CLI is not a lagging port of the web app.
- **We use it ourselves.** Our own agents and CI jobs run `smb` and `smb --mcp` daily, so a break shows up internally before it reaches anyone else.
- **A versioned API.** The CLI talks to the REST API under `/v1`. A backward-incompatible API change gets a new version path rather than a silent edit.
- **Lockstep releases.** One tag produces every distribution (crates.io, npm, PyPI, NuGet, Homebrew, Scoop, winget and the GitHub release binaries). CI gates the version of each package against the tag.
- **A track record.** Releases have been regular since 2023, and the workspace has an automated test suite that runs on every push.

## Deprecation policy

Nothing is removed without notice. A command, flag, MCP tool or config key that is going away prints a warning for at least one minor release first. Removal happens only in the next major release.

Security fixes are the exception: they can change behavior in a patch release when that is the only safe fix. See [SECURITY.md](../SECURITY.md).

## Readiness checklist

| Item | Status |
| --- | --- |
| CI green on `development` and every release channel published for the current version | Done |
| `SECURITY.md` supports the latest minor of the current major, not a pre-1.0 line | Done |
| Panics on user-driven paths removed (GitHub OAuth callback listener, runner pickers in `project update` and project setup, `migrate`). A cancelled prompt now returns an error message. | Done |
| MCP tool count in the docs matches the router (41 cloud tools) | Done |
| MCP Registry release publishes. Root cause of the recent failures: the NuGet wait loop polled up to 60 times but gave up at attempt 20, so it timed out after about 10 minutes, before NuGet finished indexing the new version. The loop now waits the full 30 minutes. | Open until a release run publishes |
| `cargo install smbcloud-cli` works without `CLI_CLIENT_SECRET` at compile time. Today the secret is baked in at build time, which makes the crates.io route a second-class install. See [Native (Cargo)](./cli-install.md). | Open |
| winget listing. The first submission is waiting on a moderator and is not a blocker, because later releases update the package through the normal update path. | Waiting, not blocking |

## Cutting 1.0.0

Once the open items are closed, from a clean `development` branch:

```sh
make custom VERSION=1.0.0
```

This bumps every crate and SDK to 1.0.0 in lockstep, commits `Release 1.0.0` and tags `v1.0.0` locally. Pushing the tag is a manual step.
