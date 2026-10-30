<p align="center">
  <img src="https://avatars.githubusercontent.com/u/89791739?s=200&v=4" alt="smbCloud Logo" width="96">
</p>

<h1 align="center">smbCloud CLI</h1>

<p align="center"><strong>smbCloud Mail, Auth, Deploy and GresIQ from your terminal, your scripts and your agents.</strong></p>

<p align="center">
  <a href="https://crates.io/crates/smbcloud-cli"><img alt="Crates.io" src="https://img.shields.io/crates/v/smbcloud-cli"></a>
  <a href="https://www.npmjs.com/package/@smbcloud/cli"><img alt="npm" src="https://img.shields.io/npm/v/@smbcloud/cli"></a>
  <a href="https://www.nuget.org/packages/SmbCloud.Cli"><img alt="NuGet" src="https://img.shields.io/nuget/v/SmbCloud.Cli"></a>
  <a href="https://github.com/smbcloudXYZ/homebrew-tap"><img alt="Homebrew" src="https://img.shields.io/badge/homebrew-tap-orange?logo=homebrew"></a>
  <a href="https://pypi.org/project/smbcloud-cli/"><img alt="PyPI" src="https://img.shields.io/pypi/v/smbcloud-cli"></a>
  <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=io.github.smbcloudXYZ/smbcloud-cli"><img alt="MCP server" src="https://badge.mcpx.dev?type=server&features=resources,tools" title="MCP server with features"></a>
  <a href="https://github.com/smbcloudXYZ/smbcloud-cli/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/smbcloudXYZ/smbcloud-cli/ci.yml?label=ci"></a>
  <a href="https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE"><img alt="Apache-2.0 License" src="https://img.shields.io/badge/license-Apache%202.0-blue.svg"></a>
</p>

Every smbCloud capability works without a browser. You reach it through the REST API first, then through `smb`, then through its MCP server, and each one covers the same operations. `smb` is one binary with no runtime to install, so a person at a terminal, a CI job and an AI agent all run the same commands against the same account.

## Install

```sh
brew tap smbcloudXYZ/tap && brew trust --tap smbcloudXYZ/tap && brew install cli
npm install -g @smbcloud/cli
cargo install smbcloud-cli
```

More ways, including .NET, pip, shell, PowerShell and pre-built binaries, are in [docs/install.md](docs/install.md).

## Use it your way

- **In the terminal.** `smb login && smb init && smb deploy` takes you from a fresh login to a first deploy. See [interfaces](docs/interfaces.md).
- **In scripts and CI.** `smb --ci` turns off prompts, so a missing answer fails fast instead of hanging. See [CI mode](docs/ci.md).
- **With AI agents.** `smb --mcp` serves the [Model Context Protocol](https://modelcontextprotocol.io) over stdio, and it is listed in the MCP Registry as `io.github.smbcloudXYZ/smbcloud-cli`. See [MCP](docs/mcp.md).
- **For mobile and TV automation.** `smb --mcp --scope automation` exposes Xcode, ControlKit, CoreDevice and adb tools. See [ControlKit](docs/controlkit.md).

## Docs

[Install](docs/install.md), [interfaces](docs/interfaces.md), [CI](docs/ci.md), [MCP](docs/mcp.md), [MCP Registry](docs/mcp-registry.md), [ControlKit](docs/controlkit.md), [AndroidKit](docs/androidkit.md), [development](docs/development.md). The index is [docs/README.md](docs/README.md).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) to get started. Contributions are welcome.

## License

[Apache-2.0](LICENSE)

## Copyright

© 2026 Seto Elkahfi ([smbCloud](https://smbcloud.xyz)). Distributed by [Splitfire AB](https://5mb.app).
