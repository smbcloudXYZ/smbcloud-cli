# MCP Registry

The smbCloud CLI's MCP server is listed in the [official MCP Registry](https://registry.modelcontextprotocol.io)
as **`io.github.smbcloudxyz/smbcloud-cli`**. The registry is a metadata index —
it doesn't host binaries — so the listing points at packages we already publish
to npm and NuGet, and clients that browse the registry can offer a one-click
install of `smb --mcp`.

For running and configuring the server itself, see [MCP Server](./mcp.md).

## What makes up the listing

| Piece | Where |
| --- | --- |
| Server metadata | [`server.json`](../server.json) at the repo root |
| npm ownership proof | `mcpName` in the generated `@smbcloud/cli` `package.json` (`npm/scripts/render-main-package.cjs`) |
| NuGet ownership proof | `<!-- mcp-name: ... -->` in `nuget/smbcloud-cli/README.md` |
| Publishing | `.github/workflows/release-mcp-registry.yml` |

The registry checks each listed package for a marker naming the server. If the
marker is missing or doesn't match `name` in `server.json`, publishing fails
with "Registry validation failed for package". The publish workflow greps for
both markers before it does anything else, so a mismatch shows up as a failed
job rather than a rejected upload.

### Why the name is `io.github.smbcloudxyz/...`

The namespace has to match the authentication method. We authenticate with
GitHub OIDC from Actions, which grants the `io.github.smbcloudxyz/*` namespace
(the reverse-DNS form of the `smbcloudXYZ` org). Publishing under
`xyz.smbcloud/*` instead would mean domain-based auth: a `v=MCPv1` TXT record on
`smbcloud.xyz` plus an Ed25519 private key stored as a repo secret. OIDC needs
no secret at all, so that's what we use.

### Which packages are listed

- **npm** — `@smbcloud/cli`, launched as `npx @smbcloud/cli --mcp`. The package
  declares a single binary (`smb`), so `npx` resolves it despite the name
  difference.
- **NuGet** — `SmbCloud.Cli`, launched as `dnx SmbCloud.Cli --mcp`.

PyPI is deliberately not listed. The `smbcloud-cli` wheel installs its
executable as `smb` (maturin `bindings = "bin"`), and `uvx smbcloud-cli` looks
for an executable matching the distribution name, so the launch command the
registry hands to clients wouldn't work. Adding PyPI would mean shipping an
extra console script named `smbcloud-cli`. Homebrew and the direct GitHub
release binaries have no registry package type at all; those install paths stay
documented in [Install](./cli-install.md).

## Releasing a new version

`server.json` carries the version twice — once for the server, once per package
— and both are updated by `make patch | minor | major` along with the rest of
the release metadata (`scripts/sync-release-version.mjs`).

After tagging and running the npm and NuGet release workflows, run the
**MCP Registry Release** workflow with the same tag:

```sh
gh workflow run release-mcp-registry.yml --ref development -f tag=v0.4.12
```

The workflow publishes the `server.json` on the branch you dispatch from, not
the one at the tag — it uploads metadata rather than building from source, and
the metadata can change after a tag is cut. The tag input only selects which
version to publish.

Order matters. The registry fetches the package metadata during publish, so
`@smbcloud/cli@<version>` and `SmbCloud.Cli <version>` must already be live —
the workflow checks for both and stops early if they aren't.

Confirm the listing afterwards:

```sh
curl "https://registry.modelcontextprotocol.io/v0.1/servers?search=io.github.smbcloudxyz/smbcloud-cli"
```

## Publishing by hand

Rarely needed, but if Actions is unavailable:

```sh
brew install mcp-publisher
mcp-publisher login github        # device flow, needs push access to smbcloudXYZ
mcp-publisher publish             # reads ./server.json
```

Versions are immutable — republishing the same version is rejected. Fixing a
bad listing means shipping a new patch version.
