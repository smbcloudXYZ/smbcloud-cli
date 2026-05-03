<!-- LOGO -->
<h1>
<p align="center">
  <img src="https://avatars.githubusercontent.com/u/89791739?s=200&v=4" alt="smbCloud Logo" width="128">
  <br>smbCloud CLI
</h1>
  <p align="center">
    Deploy to the cloud in one command.
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
    <a href="https://www.npmjs.com/package/@smbcloud/cli"><img alt="npm" src="https://img.shields.io/npm/v/@smbcloud/cli"></a>
    <a href="https://www.nuget.org/packages/SmbCloud.Cli"><img alt="NuGet" src="https://img.shields.io/nuget/v/SmbCloud.Cli"></a>
    <a href="https://pypi.org/project/smbcloud-cli/"><img alt="PyPI" src="https://img.shields.io/pypi/v/smbcloud-cli"></a>
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/smbcloudXYZ/smbcloud-cli"></a>
  </p>
</p>

## About

**`smb`** is the command-line interface for [smbCloud](https://smbcloud.xyz/) — the modern cloud deployment platform. We've eliminated the friction of cloud infrastructure so you can focus on what matters: building an incredible product.

Ship your Rust, Node.js, Ruby, or Swift app with a single, magical command.

## Install

```sh
npm install -g @smbcloud/cli
```

This package automatically downloads and installs the correct pre-built native binary for your platform.

## Quick Start

```sh
smb login
smb init
smb deploy
```

That's it. Your app is live.

## Other Installation Methods

### Homebrew (macOS & Linux)

```sh
brew tap smbcloudXYZ/tap
brew install cli
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

This is a native binary distributed via npm. The source code lives at
[github.com/smbcloudXYZ/smbcloud-cli](https://github.com/smbcloudXYZ/smbcloud-cli).
Please report bugs and feature requests there.

> Explore more on the [smbCloud Services](https://smbcloud.xyz/services) page.

## License

[Apache-2.0](https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE)

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
