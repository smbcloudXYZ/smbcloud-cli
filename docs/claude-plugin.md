# Claude plugin

smbCloud ships a plugin for Claude's plugin directory. It adds the hosted
smbCloud MCP server to Claude (claude.ai chat, Cowork, and Claude Code) so a
user can read and send mail from their smbCloud Mail inboxes and run web
searches in a conversation.

The plugin is separate from `smb --mcp`. The hosted server exposes the mail and
web search tools and signs users in with OAuth. The full tool set (projects,
deployments, tenants, Auth apps) stays in the local `smb --mcp` server, which
needs the CLI installed and only works in Claude Code. See [MCP Server](./mcp.md).

## Layout

```text
packaging/claude/smbcloud/
  .claude-plugin/
    plugin.json    manifest: name, version, license, icon, privacy policy URL
    icon.png       square PNG, 512 to 2048 px
  .mcp.json        the hosted server, type http
  README.md        the listing text
```

The plugin has no scripts, hooks, or `bin/` directory. A `bin/` directory would
stop claude.ai and Cowork from installing the plugin, and the directory holds
scripts and launchers for manual review. `.mcp.json` holds no credentials: the
user signs in through OAuth when they first use it.

`plugin.json` carries `"name": "smbcloud"`. The name is permanent after the first
release, so do not change it. The display name (`displayName`) can change.

## Versions

`plugin.json` `version` follows the release version. `scripts/sync-release-version.mjs`
updates it with the other manifests and `scripts/check-release-versions.mjs`
fails CI when it drifts, so `make patch|minor|major` needs no extra step.

The directory raises an update only when `version` goes up, so every release
that changes the plugin must go through the Makefile.

## How the directory picks up a release

There is no release workflow. The directory tracks a branch of this repository
(`main`) and scans the newest commit on it. Merging a release into `main`
publishes the new plugin version, after the directory's own scan and review.

## First submission

Submission happens in the developer portal at <https://claude.ai/directory/manage>
(**Submit new**, then **Plugin bundle**). An owner of the organization needs to do it.

1. Run `claude plugin validate packaging/claude/smbcloud` from a clean checkout of
   the branch you will submit. It must print `Validation passed`.
2. In the portal, enter the repository (`smbcloudXYZ/smbcloud-cli`), the plugin
   path (`packaging/claude/smbcloud`), and the branch to track (`main`).
3. Run the portal's own **Validate** step and fix anything marked Blocks.
4. Fill in the listing details, data handling, and compliance steps. The data
   handling answers match the README: tool arguments and results go to
   `api.smbcloud.xyz` only, nothing is stored on the user's machine.
5. Choose the update method (GitHub push webhook) and submit for review.

A submission is per plugin folder, and the limit is 10 submissions per
organization per 24 hours.

## What the directory checks

These rules caused real validation findings or are easy to break later:

- The name cannot be a reserved word (`claude`, `anthropic`, `official`, `plugin`,
  `mcp`, `test`) and cannot imply the plugin is official.
- The README needs at least 40 words outside code blocks and a statement of what
  data the plugin sends.
- `plugin.json` needs a license (or a `LICENSE` file), a privacy policy URL for
  plugins that talk to a remote server, and an icon that is a square PNG or JPEG
  between 512 and 2048 px and under 2 MB. The directory can take the icon only
  once, the first time the plugin is saved or submitted, so check it before then.
- Only text, SVG, PNG, JPEG, GIF, WebP, and fonts are allowed in the plugin folder.
  Symlinks are not allowed for loaded files.
- No real credentials in any file.
- A package launcher such as `npx` or `uvx` has to pin an exact version, and even
  then the plugin goes to manual review. That is why the plugin uses the hosted
  server instead of `npx @smbcloud/cli --mcp`.

## Requirements on the hosted server

Claude can only sign users in if `https://api.smbcloud.xyz/v1/mcp` implements the
MCP authorization flow:

- A `401` with `WWW-Authenticate: Bearer resource_metadata="..."`.
- Protected-resource metadata whose `resource` equals the MCP URL exactly.
- Authorization server metadata with PKCE `S256`, `none` among the token endpoint
  auth methods, and a registration endpoint (dynamic client registration).
- Callbacks for `https://claude.ai/api/mcp/auth_callback` and loopback
  redirects with any port for Claude Code.
- Refresh token rotation for public clients.

These live in the API repository. Test them against production before submitting:

```sh
curl -si -X POST https://api.smbcloud.xyz/v1/mcp -H 'content-type: application/json' -d '{}' | grep -i www-authenticate
curl -s https://api.smbcloud.xyz/.well-known/oauth-protected-resource/v1/mcp
curl -s https://api.smbcloud.xyz/.well-known/oauth-authorization-server
```

Then run `claude --plugin-dir packaging/claude/smbcloud`, open `/mcp`, authenticate,
and call `whoami`.

## Later

The skills in `.agents/skills/` are written for people working on this repository
and cite source files, so they do not belong in the plugin. A later version could
add user-facing skills adapted from the deploy skills (Next.js, Rust, monorepo).
