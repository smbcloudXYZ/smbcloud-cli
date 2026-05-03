<!-- LOGO -->
<h1>
<p align="center">
  <img src="https://avatars.githubusercontent.com/u/89791739?s=200&v=4" alt="smbCloud Logo" width="128">
  <br>smbCloud CLI
</h1>
  <p align="center">
    Deploy with smbCloud from your terminal.
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
    <a href="https://crates.io/crates/smbcloud-cli"><img alt="Crates.io" src="https://img.shields.io/crates/v/smbcloud-cli"></a>
    <a href="https://www.npmjs.com/package/@smbcloud/cli"><img alt="npm" src="https://img.shields.io/npm/v/@smbcloud/cli"></a>
    <a href="https://www.nuget.org/packages/SmbCloud.Cli"><img alt="NuGet" src="https://img.shields.io/nuget/v/SmbCloud.Cli"></a>
    <a href="https://github.com/smbcloudXYZ/homebrew-tap"><img alt="Homebrew" src="https://img.shields.io/badge/homebrew-tap-orange?logo=homebrew"></a>
    <a href="https://pypi.org/project/smbcloud-cli/"><img alt="PyPI" src="https://img.shields.io/pypi/v/smbcloud-cli"></a>
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/smbcloudXYZ/smbcloud-cli"></a>
  </p>
</p>

## About

**`smb`** is the command-line interface for [smbCloud](https://smbcloud.xyz/).

Install it with `pip`, then run the native `smb` binary for your platform.

## Install

```sh
pip install smbcloud-cli
```

This package installs the native `smb` executable for your platform. You do not need Node.js or Docker.

## Quick Start

```sh
smb login
smb init
smb deploy
```

That gets you from login to first deploy.

## Other Installation Methods

### Homebrew (macOS & Linux)

```sh
brew tap smbcloudXYZ/tap
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

### Shell (macOS / Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/smbcloudXYZ/smbcloud-cli/main/install-unix.sh | sh
```

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/smbcloudXYZ/smbcloud-cli/main/install-windows.sh | iex
```

Or grab a pre-built binary from the [Releases](https://github.com/smbcloudXYZ/smbcloud-cli/releases) page.

## Documentation

Full documentation is available at [smbcloud.xyz/posts](https://smbcloud.xyz/posts).

## Platform Support

This package ships pre-built native binaries for:

| Platform      | Architecture |
| ------------- | ------------ |
| macOS         | arm64, x64   |
| Linux (glibc) | arm64, x64   |
| Windows       | arm64, x64   |

## Source & Issues

This package ships the native `smb` binary through PyPI.
Source code and issue tracker:
[github.com/smbcloudXYZ/smbcloud-cli](https://github.com/smbcloudXYZ/smbcloud-cli).

See the [smbCloud documentation](https://smbcloud.xyz/posts) for setup and deployment guides.

## License

[Apache-2.0](https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE)

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
