# xcrs for .NET

<!-- mcp-name: io.github.smbcloudXYZ/xcrs -->

`xcrs` drives mobile and TV apps — on iOS simulators, physical Apple devices, and
adb-connected Android phones, tablets, emulators, and TVs — through the
[Model Context Protocol](https://modelcontextprotocol.io/). It is the standalone
automation profile of the [smbCloud CLI](https://smbcloud.xyz/).

## Install

```sh
dotnet tool install --global SmbCloud.Xcrs
```

## Update

```sh
dotnet tool update --global SmbCloud.Xcrs
```

## Quick start

Run it as an MCP server over stdio and point your MCP client at it:

```sh
xcrs --mcp
```

## Platform support

This .NET tool bundles native `xcrs` binaries for:

- macOS `arm64`, `x64`
- Linux `arm64`, `x64` (glibc)
- Windows `arm64`, `x64`

The full Apple automation surface requires macOS with Xcode command-line tools.
Android automation over `adb` works from any platform.

## Other installation methods

- Cargo: `cargo install xcrs`
- npm: `npm install -g @smbcloud/xcrs`
- pip: `pip install xcrs`

## Source

- Repository: <https://github.com/smbcloudXYZ/smbcloud-cli>
- Documentation: <https://smbcloud.xyz/posts>
- Issues: <https://github.com/smbcloudXYZ/smbcloud-cli/issues>

## License

Apache-2.0

## Copyright

© 2026 [Splitfire AB](https://5mb.app) ([smbCloud](https://smbcloud.xyz)).
