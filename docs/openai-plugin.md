# OpenAI plugin (ChatGPT and Codex)

smbCloud ships a plugin for OpenAI's plugin directory, which ChatGPT and Codex
share. It is listed as **smbCloud Mail** in the Communication category and
bundles the hosted MCP server at `https://api.smbcloud.xyz/v1/mcp`: the user's
smbCloud Mail inboxes plus web search, with OAuth sign-in.

It is a separate package from the [Claude plugin](./claude-plugin.md) because
OpenAI uses its own manifest fields and takes an upload instead of following a
branch.

## Layout

```text
packaging/openai/smbcloud/
  plugin.json      portable Agent Plugins manifest, OpenAI fields under extensions["com.openai"]
  mcp.json         one server: streamable HTTP, OAuth with dynamic client registration
  assets/logo.png  square 866 px PNG, used as both logo and composerIcon
```

The listing must not tell users to add another MCP server, a connector, or the
local `smb --mcp` server. The Claude directory refused a plugin for exactly that,
and OpenAI's guidelines forbid steering the model or the user to other plugins.
Docs outside `packaging/openai/` can still describe `smb --mcp`.

`mcp.json` puts the OAuth settings at `mcpServers.smbcloud.extensions["com.openai"].auth`,
as OpenAI's build docs show. The generic Agent Plugins schema 1.0.0 does not
allow `extensions` on a server entry. If the portal ever rejects it, drop the
`extensions` block: the server's OAuth discovery already tells ChatGPT how to
register and sign in.

## Versions and packaging

`plugin.json` `version` follows the release version through
`scripts/sync-release-version.mjs`, and `scripts/check-release-versions.mjs`
fails on drift.

Build the upload with:

```sh
scripts/package-openai-plugin.sh
```

It writes `dist/smbcloud-openai-plugin-<version>.zip` with `plugin.json`,
`mcp.json` and `assets/` at the archive root, and refuses to build when the
package contains a client secret, reviewer credentials, server headers, or an
instruction to add another MCP server.

## How updates reach users

- Changes to `plugin.json`, the icon, or the listing need a new ZIP upload, which
  becomes a new package version with its own review.
- Changes to the hosted MCP server need no upload. OpenAI scans the server daily
  and applies eligible changes after its automated checks; a rescan can be
  requested from the portal after a deploy. New tools stay unavailable until
  they are approved.
- The MCP server URL cannot be changed through the update flow.

## Requirements on the hosted server

- Every tool sets `readOnlyHint`, `destructiveHint` and `openWorldHint`, and has a
  `title`. Without them the submission is blocked (`annotations_required`).
- OAuth discovery: protected resource metadata, authorization server metadata
  with PKCE `S256`, and a registration endpoint. ChatGPT registers itself through
  dynamic client registration.
- Domain verification: `https://api.smbcloud.xyz/.well-known/openai-apps-challenge`
  returns the token from the portal as plain text. The API serves the value of
  `OPENAI_APPS_CHALLENGE`, so the token is set in the API's environment, not
  committed.

## Submitting

At https://platform.openai.com/plugins, as an organization owner or a member
with Apps Management Write, after the organization's identity is verified:

1. Upload the ZIP and fix any Metadata & Skills findings.
2. Connect the MCP server: URL `https://api.smbcloud.xyz/v1/mcp`, authentication
   OAuth. The portal shows a domain challenge token. Set `OPENAI_APPS_CHALLENGE`
   on the API server, then Verify Domain. Connect, sign in on the smbCloud consent
   page, and wait for the tool scan.
3. Review details: reviewer credentials for an smbCloud account that has a
   mailbox with sample mail and no MFA, the demo video URL, the test cases
   (prefilled from `plugin.json`), and a justification for each tool annotation.
4. Submit for review and complete the attestations. Feedback arrives by email.
5. After approval, publish from the portal.

### Annotation justifications

Paste these into the portal, one per tool.

| Tool | Hints | Justification |
|---|---|---|
| `whoami` | read-only, not destructive, closed world | Returns the signed-in user's email, mailbox count and mail plan. It changes nothing and reads only the user's own account. |
| `list_mailboxes` | read-only, not destructive, closed world | Lists the user's own mailboxes. No writes, and no data outside the account. |
| `list_messages` | read-only, not destructive, closed world | Lists received messages in one of the user's mailboxes. No writes, and only the user's own mail. |
| `get_message` | read-only, not destructive, closed world | Reads one message from the user's own mailbox. No writes. |
| `search_messages` | read-only, not destructive, closed world | Searches the user's own mailbox by subject, sender or text. No writes. |
| `list_sent_messages` | read-only, not destructive, closed world | Lists messages the user sent from their own mailbox. No writes. |
| `send_message` | writes, destructive, open world | Sends an email from the user's mailbox. A sent email cannot be recalled, and the recipients can be any address outside smbCloud. Sends are rate limited per user. |
| `web_search` | read-only, not destructive, open world | Searches the public web through a third-party search provider and returns titles, URLs and snippets. It writes nothing. |
