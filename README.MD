<!-- LOGO -->
<h1>
<p align="center">
  <img src="https://avatars.githubusercontent.com/u/89791739?s=200&v=4" alt="smbCloud Logo" width="128">
  <br>smbCloud CLI
</h1>
  <p align="center">
    smbCloud Mail, Auth, Deploy, and GresIQ from your terminal or your AI agent.
    <br />
    <br />
    <a href="https://smbcloud.xyz/">Website</a>
    ·
    <a href="./docs">Documentation</a>
    ·
    <a href="CONTRIBUTING.md">Contributing</a>
    ·
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/releases">Releases</a>
  </p>
  <p align="center">
    <a href="https://crates.io/crates/smbcloud-cli"><img alt="Crates.io" src="https://img.shields.io/crates/v/smbcloud-cli"></a>
    <a href="https://www.npmjs.com/package/@smbcloud/cli"><img alt="npm" src="https://img.shields.io/npm/v/@smbcloud/cli"></a>
    <a href="https://www.nuget.org/packages/SmbCloud.Cli"><img alt="NuGet" src="https://img.shields.io/nuget/v/SmbCloud.Cli"></a>
    <a href="https://github.com/smbcloudXYZ/homebrew-tap"><img alt="Homebrew" src="https://img.shields.io/badge/homebrew-tap-orange?logo=homebrew"></a>
    <a href="https://pypi.org/project/smbcloud-cli/"><img alt="PyPI" src="https://img.shields.io/pypi/v/smbcloud-cli"></a>
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE"><img alt="Apache-2.0 License" src="https://img.shields.io/badge/license-Apache%202.0-blue.svg"></a>
  </p>
</p>

## About

**`smb`** is the command-line interface for [smbCloud](https://smbcloud.xyz/). It is
also an MCP server, so Claude, Cursor, and other AI agents can run the same operations
you do.

Set up email domains and inbox routing on [smbCloud Mail](https://smbcloudmail.com),
manage authentication apps on [smbCloud Auth](https://smbcloudauth.com), and deploy
Rust, Node.js, Ruby, or Swift apps. One binary, no runtime to install.

## Install

If you want the quickest setup, use a pre-built native binary.

### Homebrew (macOS & Linux)

```sh
brew tap smbcloudXYZ/tap && brew trust --tap smbcloudXYZ/tap
brew install cli
```

### npm

```sh
npm install -g @smbcloud/cli
```

### .NET tool

```sh
dotnet tool install --global SmbCloud.Cli
```

### pip

```sh
pip install smbcloud-cli
```

### Shell (macOS / Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/smbcloudXYZ/smbcloud-cli/main/install-unix.sh | sh
```

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/smbcloudXYZ/smbcloud-cli/main/install-windows.sh | iex
```

Or grab a pre-built binary from the [Releases](https://github.com/smbcloudXYZ/smbcloud-cli/releases) page.

## Quick Start

```sh
smb login
smb init
smb deploy
```

That gets you from login to first deploy.

## MCP server

`smb --mcp` speaks the [Model Context Protocol](https://modelcontextprotocol.io) over
stdio, exposing 30 tools across smbCloud Mail, Auth, projects, and deployments. It is
listed in the official MCP Registry as `io.github.smbcloudXYZ/smbcloud-cli`.

Claude Code:

```sh
claude mcp add smbcloud -- smb --mcp
```

Claude Desktop, Cursor, and anything else that takes a JSON config:

```json
{
  "mcpServers": {
    "smbcloud": {
      "command": "npx",
      "args": ["@smbcloud/cli", "--mcp"]
    }
  }
}
```

Auth comes from `smb login`, so a terminal session and an agent session share the same
token and the same selected project. See [`docs/mcp.md`](./docs/mcp.md) for the full tool
reference.

## CI / non-interactive mode

Pass `--ci` (or set `SMB_CI=1`, or run under any provider that sets `CI`) to
disable interactive prompts. Confirmations take their default; anything that
truly needs input fails fast with a clear message instead of hanging on a
missing TTY.

```sh
smb --ci deploy
```

For example, the [AircraftsHub](https://aircraftshub.5mb.app) monorepo deploys
its Next.js web app with a single non-interactive command — its
`.smb/config.toml` pins the project, source, and runtime, so `smb --ci deploy`
needs no prompts. See [`docs/ci.md`](./docs/ci.md) for token setup and the full
behavior reference.

## Documentation

See the [`docs/`](./docs) directory for guides on authentication, project configuration, and deployment workflows.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) to get started. Contributions are welcome.

For the broader product, see [smbCloud](https://smbcloud.xyz/) and the [deployment docs](https://smbcloud.xyz/posts).

## License

[Apache-2.0](LICENSE)

## Copyright

© 2026 [Splitfire AB](https://5mb.app) ([smbCloud](https://smbcloud.xyz)).
