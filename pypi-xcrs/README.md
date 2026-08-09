<!-- LOGO -->
<h1>
<p align="center">
  <img src="https://avatars.githubusercontent.com/u/89791739?s=200&v=4" alt="smbCloud Logo" width="128">
  <br>xcrs
</h1>
  <p align="center">
    Cross-platform mobile and TV app automation over MCP.
    <br />
    <br />
    <a href="https://smbcloud.xyz/">Website</a>
    ·
    <a href="https://smbcloud.xyz/posts">Documentation</a>
    ·
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/releases">Releases</a>
    ·
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/issues">Issues</a>
  </p>
  <p align="center">
    <a href="https://pypi.org/project/xcrs/"><img alt="PyPI" src="https://img.shields.io/pypi/v/xcrs"></a>
    <a href="https://www.npmjs.com/package/@smbcloud/xcrs"><img alt="npm" src="https://img.shields.io/npm/v/@smbcloud/xcrs"></a>
    <a href="https://www.nuget.org/packages/SmbCloud.Xcrs"><img alt="NuGet" src="https://img.shields.io/nuget/v/SmbCloud.Xcrs"></a>
    <a href="https://crates.io/crates/xcrs"><img alt="Crates.io" src="https://img.shields.io/crates/v/xcrs"></a>
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/smbcloudXYZ/smbcloud-cli"></a>
  </p>
</p>

## About

**`xcrs`** drives mobile and TV apps — on iOS simulators, physical Apple
devices, and adb-connected Android phones, tablets, emulators, and TVs — through
the [Model Context Protocol](https://modelcontextprotocol.io/). It is the
standalone automation profile of the [smbCloud CLI](https://smbcloud.xyz/); the
same tools are also available via `smb --mcp --scope automation`.

This package installs the native `xcrs` executable for your platform directly —
no Node.js, no Docker, no runtime dependencies.

## Install

```sh
pip install xcrs
```

Or run it without installing:

```sh
uvx xcrs --mcp
```

## Quick start

Run it as an MCP server over stdio and point your MCP client at it:

```sh
xcrs --mcp
```

**Recommended flow:** `device_list` → `device_select` → `device_capabilities`,
then inspect with `screen_capture` / `ui_describe` and drive with the `input_*`
tools.

## Other installation methods

- Cargo: `cargo install xcrs`
- npm: `npm install -g @smbcloud/xcrs`
- NuGet: `dotnet tool install --global SmbCloud.Xcrs`

## Platform support

| Platform      | Architecture |
| ------------- | ------------ |
| macOS         | arm64, x64   |
| Linux (glibc) | arm64, x64   |
| Windows       | x64          |

The full Apple automation surface (`ui_describe`, `ui_element_list`, simulator
control) requires macOS with Xcode command-line tools. Android automation over
`adb` works from any platform.

## Source

- Repository: <https://github.com/smbcloudXYZ/smbcloud-cli>
- Documentation: <https://smbcloud.xyz/posts>
- Issues: <https://github.com/smbcloudXYZ/smbcloud-cli/issues>

## License

Apache-2.0

## Copyright

© 2026 [Splitfire AB](https://5mb.app) ([smbCloud](https://smbcloud.xyz)).
