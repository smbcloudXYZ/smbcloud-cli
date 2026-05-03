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

**`smb`** is the command-line interface for [smbCloud](https://smbcloud.xyz/).

Deploy Rust, Node.js, Ruby, or Swift apps from your terminal.

## Install

If you want the quickest setup, use a pre-built native binary.

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

## Documentation

See the [`docs/`](./docs) directory for guides on authentication, project configuration, and deployment workflows.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) to get started. Contributions are welcome.

For the broader product, see [smbCloud](https://smbcloud.xyz/) and the [deployment docs](https://smbcloud.xyz/posts).

## License

[Apache-2.0](LICENSE)

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
